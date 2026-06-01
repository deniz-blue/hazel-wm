use std::error::Error as StdError;
use std::fmt;

use miette::Report;

pub struct UnsafeSendDiagnostic<T>(pub T);

unsafe impl<T> Send for UnsafeSendDiagnostic<T> {}
unsafe impl<T> Sync for UnsafeSendDiagnostic<T> {}

impl<T: StdError> std::error::Error for UnsafeSendDiagnostic<T> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

impl<T: fmt::Display> fmt::Display for UnsafeSendDiagnostic<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Debug> fmt::Debug for UnsafeSendDiagnostic<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: StdError + 'static> miette::Diagnostic for UnsafeSendDiagnostic<T> {}

pub trait IntoDiagnostic<T, E> {
    fn into_diagnostic(self) -> Result<T, Report>;
}

impl<T, E: std::error::Error + 'static> IntoDiagnostic<T, E> for Result<T, E> {
    fn into_diagnostic(self) -> Result<T, Report> {
        self.map_err(|e| UnsafeSendDiagnostic(e).into())
    }
}
