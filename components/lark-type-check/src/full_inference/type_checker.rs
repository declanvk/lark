use crate::full_inference::apply_perm::ApplyPerm;
use crate::full_inference::constraint::Constraint;
use crate::full_inference::constraint::ConstraintAt;
use crate::full_inference::perm::Perm;
use crate::full_inference::perm::PermData;
use crate::full_inference::perm::PermVar;
use crate::full_inference::FullInference;
use crate::full_inference::FullInferenceTables;
use crate::results::TypeCheckResults;
use crate::substitute::Substitution;
use crate::substitute::SubstitutionDelegate;
use crate::TypeCheckDatabase;
use crate::TypeChecker;
use crate::TypeCheckerFamilyDependentExt;
use crate::TypeCheckerVariableExt;
use lark_collections::FxIndexSet;
use lark_entity::Entity;
use lark_hir as hir;
use lark_indices::IndexVec;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_ty::declaration;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclaredPermKind;
use lark_ty::map_family::Map;
use lark_ty::BaseKind;
use lark_ty::Erased;
use lark_ty::GenericKind;
use lark_ty::Generics;
use lark_ty::PermKind;
use lark_ty::ReprKind;
use lark_ty::Ty;

/// The full-inference-specific data stored in the type-checker when
/// doing full inference.
#[derive(Default)]
crate struct FullInferenceStorage {
    /// Set of all permission veriables created. Right now we don't
    /// keep any information about them in particular.
    perm_vars: IndexVec<PermVar, ()>,

    /// Constraints we have created during type-checking thus far.
    crate constraints: FxIndexSet<ConstraintAt>,

    /// Results we have generated thus far.
    crate results: TypeCheckResults<FullInference>,
}

impl FullInferenceStorage {
    crate fn new_inferred_perm(&mut self, tables: &dyn AsRef<FullInferenceTables>) -> Perm {
        PermData::Inferred(self.perm_vars.push(())).intern(tables)
    }

    crate fn add_constraint(&mut self, cause: impl Into<hir::MetaIndex>, constraint: Constraint) {
        self.constraints.insert(ConstraintAt {
            cause: cause.into(),
            constraint,
        });
    }
}

impl<DB> TypeCheckerFamilyDependentExt<FullInference>
    for TypeChecker<'me, DB, FullInference, FullInferenceStorage>
where
    DB: TypeCheckDatabase,
{
    fn require_assignable(&mut self, expression: hir::Expression, place_ty: Ty<FullInference>) {
        let value_ty = self.storage.results.ty(expression);

        // When assigning a value into a place, we do not *have* to
        // transfer the full permissions of that value into the
        // place. So create a permission variable for the amount of
        // access and use it to modify the value access ty.
        let perm_access = self.storage.new_inferred_perm(&self.f_tables);
        let value_access_ty = self.apply_access_perm(expression.into(), perm_access, value_ty);

        // Record the permission used at `expression` for later.
        self.storage
            .results
            .access_permissions
            .insert(expression, perm_access);

        self.equate(expression, value_access_ty, place_ty)
    }

    fn substitute<M>(
        &mut self,
        _location: impl Into<hir::MetaIndex>,
        generics: &Generics<FullInference>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, FullInference>,
    {
        value.map(&mut Substitution::new(self, generics))
    }

    fn apply_owner_perm(
        &mut self,
        location: impl Into<hir::MetaIndex>,
        owner_perm: Perm,
        field_ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        self.apply_access_perm(location.into(), owner_perm, field_ty)
    }

    fn record_variable_ty(&mut self, var: hir::Variable, ty: Ty<FullInference>) {
        self.storage.results.record_ty(var, ty);
    }

    fn record_expression_ty(
        &mut self,
        expr: hir::Expression,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        self.storage.results.record_ty(expr, ty);
        ty
    }

    fn record_place_ty(&mut self, place: hir::Place, ty: Ty<FullInference>) -> Ty<FullInference> {
        self.storage.results.record_ty(place, ty);
        ty
    }

    fn request_variable_ty(&mut self, var: hir::Variable) -> Ty<FullInference> {
        self.storage.results.opt_ty(var).unwrap_or_else(|| {
            let ty = self.new_variable();
            self.storage.results.record_ty(var, ty);
            ty
        })
    }

    fn record_entity(&mut self, index: hir::Identifier, entity: Entity) {
        self.storage.results.record_entity(index, entity);
    }

    fn record_entity_and_get_generics(
        &mut self,
        index: impl Into<hir::MetaIndex>,
        entity: Entity,
    ) -> Generics<FullInference> {
        let index: hir::MetaIndex = index.into();
        self.storage.results.record_entity(index, entity);
        let generics = self.inference_variables_for(entity);
        self.storage.results.record_generics(index, &generics);
        generics
    }
}

impl<DB> TypeCheckerVariableExt<FullInference, Ty<FullInference>>
    for TypeChecker<'me, DB, FullInference, FullInferenceStorage>
where
    DB: TypeCheckDatabase,
{
    fn new_variable(&mut self) -> Ty<FullInference> {
        Ty {
            repr: Erased,
            perm: self.storage.new_inferred_perm(&self.f_tables),
            base: self.unify.new_inferable(),
        }
    }

    fn equate(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        ty1: Ty<FullInference>,
        ty2: Ty<FullInference>,
    ) {
        let cause = cause.into();

        let Ty {
            repr: Erased,
            perm: perm1,
            base: base1,
        } = ty1;
        let Ty {
            repr: Erased,
            perm: perm2,
            base: base2,
        } = ty2;

        self.storage
            .add_constraint(cause, Constraint::PermEquate { a: perm1, b: perm2 });

        match self.unify.unify(cause, base1, base2) {
            Ok(()) => {}

            Err((data1, data2)) => {
                match (data1.kind, data2.kind) {
                    (BaseKind::Error, _) => {
                        self.propagate_error(cause, &data2.generics);
                        return;
                    }
                    (_, BaseKind::Error) => {
                        self.propagate_error(cause, &data1.generics);
                        return;
                    }
                    _ => {}
                }

                if data1.kind != data2.kind {
                    self.record_error("Mismatched types", cause);
                    return;
                }

                for (generic1, generic2) in data1.generics.iter().zip(&data2.generics) {
                    match (generic1, generic2) {
                        (GenericKind::Ty(g1), GenericKind::Ty(g2)) => {
                            self.equate(cause, g1, g2);
                        }
                    }
                }
            }
        }
    }
}

impl<DB, S> AsRef<FullInferenceTables> for TypeChecker<'_, DB, FullInference, S>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &FullInferenceTables {
        &self.f_tables
    }
}

impl<DB> SubstitutionDelegate<FullInference>
    for TypeChecker<'me, DB, FullInference, FullInferenceStorage>
where
    DB: TypeCheckDatabase,
{
    fn as_f_tables(&self) -> &FullInferenceTables {
        self.as_ref()
    }

    fn map_repr_perm(&mut self, _repr: ReprKind, perm: declaration::Perm) -> (Erased, Perm) {
        let perm = self.map_perm(perm);

        (Erased, perm)
    }

    fn map_perm(&mut self, perm: declaration::Perm) -> Perm {
        match perm.untern(self) {
            DeclaredPermKind::Own => PermData::Known(PermKind::Own).intern(self),
        }
    }

    fn apply_repr_perm(
        &mut self,
        _repr: ReprKind,
        perm: declaration::Perm,
        ty: Ty<FullInference>,
    ) -> Ty<FullInference> {
        match perm.untern(self) {
            DeclaredPermKind::Own => {
                // If you have `own T` and you substitute `U` for `T`,
                // the result is just `U`.
                ty
            }
        }
    }
}
