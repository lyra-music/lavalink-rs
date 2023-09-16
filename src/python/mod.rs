pub mod client;
pub mod model;
pub mod node;
pub mod player;

use pyo3::prelude::*;

#[pyfunction]
pub fn test(_user_id: model::PyUserId) -> PyResult<String> {
    Ok("a".to_string())
}

macro_rules! with_getter_setter {
    (
        #[pymethods]
        impl $T:ty {
            getter_setter!(
                $( ($x:ident, $t:ty) ),* $(,)?
            );

            $($rest:tt)*
        }
    ) => (::paste::paste! {
        #[pymethods]
        impl $T {
            $(
                #[getter]
                fn [< get_ $x >](&self) -> ::pyo3::PyResult<$t> {
                    Ok(self.$x.clone())
                }

                #[setter]
                fn [< set_ $x >](&mut self, value: $t) -> ::pyo3::PyResult<()> {
                    self.$x = value;
                    Ok(())
                }
            )*

            $($rest)*
        }
    });
}

pub(crate) use with_getter_setter;
