use crate::map_family::FamilyMapper;
use crate::Ty;
use crate::TypeFamily;
use derive_new::new;

#[derive(new)]
pub struct Identity<'me, DB> {
    db: &'me DB,
}

impl<DB, F> FamilyMapper<F, F> for Identity<'_, DB>
where
    DB: AsRef<F::InternTables>,
    F: TypeFamily,
{
    fn map_ty(&mut self, ty: Ty<F>) -> Ty<F> {
        ty
    }

    fn map_placeholder(&mut self, placeholder: F::Placeholder) -> F::Placeholder {
        placeholder
    }
}

impl<DB> AsRef<DB> for Identity<'_, DB> {
    fn as_ref(&self) -> &DB {
        self.db
    }
}
