use bincode::{Decode, Encode};

// Message envelope for encoding multiplexed request/response
// pairs in a duplex stream  
#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct Message<T> {
    // used to correlate request/response pairs
    pub id: u32,
    pub payload: T
}