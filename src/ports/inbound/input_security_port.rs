use crate::security::ProcessResult;

pub trait InputSecurityPort: Send + Sync {
    fn process_input(&self, input: &str) -> ProcessResult;
}

impl InputSecurityPort for crate::security::SecurityService {
    fn process_input(&self, input: &str) -> ProcessResult {
        crate::security::SecurityService::process_input(self, input)
    }
}
