use pinocchio::error::ProgramError;

#[derive(Debug, Clone, PartialEq)]
pub enum MosaicError {
    PayerMustEqualSigner = 6000,
    RootAccountMustBeWrittable,
    RootAccountMustBeInitialized,
    RootAccountMustNotBeInitialized,
    RootAccountIncorrectOwner,
    SigningSessionAccountMustBeWritable,
    SigningSessionAccountMustBeInitialized,
    SigningSessionAccountMustNotBeInitialized,
    SigningSessionAccountIncorrectOwner,
    SigningSessionPhaseIncorrect,
    DestinationProgramMissmatch,
    SigningSessionPhaseAtFinalStage,
    SigningSessionSignerAlreadyApproved,
    SignerIsNotOperator,
    SigningSessionIdMustEqualRootLastId,
    ApprovalsDidNotReachThreshold,
    ProvidedDestinationProgramMismatchWithRootDestinationProgram,
    ThresholdCanNotBeHigherThanLenOfOperators,
    OperatorsCountMustBePositive,
    ThresholdCanNotBeZero,
    ReachedOperatorsLimit,
}

impl std::fmt::Display for MosaicError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MosaicError::PayerMustEqualSigner => write!(f, "payer and signer must equal"),
            MosaicError::RootAccountMustBeWrittable => {
                write!(f, "root account must be writtable")
            }
            MosaicError::RootAccountMustBeInitialized => {
                write!(f, "root account must be initialized")
            }
            MosaicError::RootAccountMustNotBeInitialized => {
                write!(f, "root account must be not initialized")
            }
            MosaicError::RootAccountIncorrectOwner => {
                write!(f, "root account owner must equal program id")
            }
            MosaicError::SigningSessionAccountMustBeWritable => {
                write!(f, "signing session account must be writtable")
            }
            MosaicError::SigningSessionAccountMustBeInitialized => {
                write!(f, "signing session account must be initialized")
            }
            MosaicError::SigningSessionAccountMustNotBeInitialized => {
                write!(f, "signing session account must not be initialized")
            }
            MosaicError::SigningSessionAccountIncorrectOwner => {
                write!(f, "signing session account owner must equal program id")
            }
            MosaicError::SigningSessionPhaseIncorrect => {
                write!(f, "signing session phase incorrect")
            }
            MosaicError::DestinationProgramMissmatch => {
                write!(
                    f,
                    "provided destination program address do not match registered in root"
                )
            }
            MosaicError::SigningSessionPhaseAtFinalStage => {
                write!(f, "can't progress pase over executed state")
            }
            MosaicError::SigningSessionSignerAlreadyApproved => {
                write!(f, "signer already casted approval for the session")
            }
            MosaicError::SignerIsNotOperator => {
                write!(f, "signer isn't recognised as known operator")
            }
            MosaicError::SigningSessionIdMustEqualRootLastId => {
                write!(f, "root last id and session id must equal")
            }
            MosaicError::ApprovalsDidNotReachThreshold => {
                write!(
                    f,
                    "there is not enough approvals to wrap the session as approved"
                )
            }
            MosaicError::ProvidedDestinationProgramMismatchWithRootDestinationProgram => {
                write!(
                    f,
                    "root pda destination program should match provided program id for cpi"
                )
            }
            MosaicError::ThresholdCanNotBeHigherThanLenOfOperators => {
                write!(f, "threshold should not be higher than operators count")
            }
            MosaicError::OperatorsCountMustBePositive => {
                write!(f, "there must be at least one operator")
            }
            MosaicError::ThresholdCanNotBeZero => {
                write!(f, "threshold can not be zero")
            }
            MosaicError::ReachedOperatorsLimit => {
                write!(f, "max operators limit reached")
            }
        }
    }
}

impl From<MosaicError> for ProgramError {
    fn from(error: MosaicError) -> ProgramError {
        ProgramError::Custom(error as u32)
    }
}
