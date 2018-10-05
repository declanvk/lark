use crate::ty::declaration::Declaration;
use crate::ty::interners::HasTyInternTables;
use crate::ty::interners::TyInternTables;
use crate::ty::map_family::FamilyMapper;
use crate::ty::map_family::Map;
use crate::ty::BoundVar;
use crate::ty::BoundVarOr;
use crate::ty::Erased;
use crate::ty::Generic;
use crate::ty::Ty;
use crate::ty::TypeFamily;

crate struct Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    intern_tables: &'me TyInternTables,
    values: &'me V,
}

impl<T, V> Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    crate fn new(intern_tables: &'me dyn HasTyInternTables, values: &'me V) -> Self {
        Substitution {
            intern_tables: intern_tables.ty_intern_tables(),
            values,
        }
    }
}

impl<T, V> HasTyInternTables for Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    fn ty_intern_tables(&self) -> &TyInternTables {
        &self.intern_tables
    }
}

impl<T, V> FamilyMapper<Declaration, T> for Substitution<'me, T, V>
where
    T: TypeFamily<Perm = Erased>,
    V: std::ops::Index<BoundVar, Output = Generic<T>>,
{
    fn map_ty(&mut self, ty: Ty<Declaration>) -> Ty<T> {
        let Ty { perm: Erased, base } = ty;

        match self.untern(base) {
            BoundVarOr::BoundVar(var) => self.values[var].assert_ty(),

            BoundVarOr::Known(base_data) => {
                let base_data1 = base_data.map(self);
                Ty {
                    perm: Erased,
                    base: T::intern_base_data(self, base_data1),
                }
            }
        }
    }
}