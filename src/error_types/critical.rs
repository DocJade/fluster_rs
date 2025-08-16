// Critical errors are errors that we cannot recover from without some sort of higher intervention.
// Returning this error type means you've done all you possibly can, and need saving at a higher level, or
// we are in a unrecoverable state.

use std::process::exit;

use thiserror::Error;
use log::error;

use crate::error_types::drive::InvalidDriveReason;

#[derive(Debug, Error, PartialEq)]
/// Use this error type if an error happens that you are unable to
/// recover from without intervention.
/// 
/// Creating critical errors is a last resort. Whatever error that was causing
/// your failure must be passed in.
pub enum CriticalError {
    #[error("Reading from the floppy disk is not working.")]
    FloppyReadFailure(std::io::ErrorKind, Option<i32>),
    #[error("Writing to the floppy disk is not working.")]
    FloppyWriteFailure(std::io::ErrorKind, Option<i32>),
    #[error("The floppy drive is inaccessible for some reason.")]
    DriveInaccessible(InvalidDriveReason),
}




//
// =========
// Attempt to recover
// =========
//

impl CriticalError {
    /// Try to recover from a critical error.
    /// 
    /// Returns nothing, since if recovery fails, fluster has shut down.
    /// If this function completes successfully, you can re-attempt the operation that resulted in the critical error.
    /// This should only be called once per operation, if you are consistently calling attempt_recovery, there is a deeper
    /// issue that you must address.
    pub(crate) fn attempt_recovery(self) {
        go_attempt_recovery(self)
    }
}


fn go_attempt_recovery(error: CriticalError) {

    // Critical recovery is not allowed in tests.
    if cfg!(test) {
        panic!("Tried to recover from a critical error! {error:#?}");
    }



    // None of that worked. We must give up.
    // .o7
    error!("Critical error recovery has failed.");
    error!("{error:#?}");
    println!("Fluster! has encountered an unrecoverable error, and must shut down.\nGoodbye.");
    exit(-1);
}