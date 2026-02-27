//! Exit code logic for the downloader process.
//!
//! Single responsibility: map completion/failure counts to the process exit outcome.

use crate::ProcessExit;

/// Determines the process exit outcome from completed and failed download counts.
pub(crate) fn determine_exit_outcome(completed: usize, failed: usize) -> ProcessExit {
    if failed == 0 {
        ProcessExit::Success
    } else if completed > 0 {
        ProcessExit::Partial
    } else {
        ProcessExit::Failure
    }
}

#[cfg(test)]
mod tests {
    use super::determine_exit_outcome;
    use crate::ProcessExit;

    #[test]
    fn test_exit_outcome_success_when_no_failures() {
        assert_eq!(determine_exit_outcome(3, 0), ProcessExit::Success);
    }

    #[test]
    fn test_exit_outcome_success_when_zero_completed_zero_failed() {
        assert_eq!(determine_exit_outcome(0, 0), ProcessExit::Success);
    }

    #[test]
    fn test_exit_outcome_partial_when_mixed() {
        assert_eq!(determine_exit_outcome(2, 1), ProcessExit::Partial);
    }

    #[test]
    fn test_exit_outcome_failure_when_all_failed() {
        assert_eq!(determine_exit_outcome(0, 2), ProcessExit::Failure);
    }
}
