#[macro_use] extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Message {
    id: u64,
    title: String,
    body: String,
    attachment_url: String,
    created_at: u64,
    updated_at: Option<u64>,

}

impl Storable for Message {
    fn to_bytes (&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes (byte: std::borrow::Cow<[u8]>) -> Self {
        Decode!(byte.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Message {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false; 
}


thread_local!{
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>>= RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    static ID_COUNTER: RefCell<IdCell> = RefCell::new(IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0).expect("Failed to create a ID_COUNTER"));
    static STORAGE: RefCell<StableBTreeMap<u64, Message, Memory>> = RefCell::new(StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))));
}


#[derive(candid::CandidType, Clone, Deserialize, Serialize, Default)]
struct MessagePayload {
    title: String,
    body: String,
    attachment_url: String,
}

#[derive(candid::CandidType, Serialize, Deserialize)]
enum Error {
    NotFound {msg: String}
}

fn _get_message (id: &u64) -> Option<Message> {
    STORAGE.with(|s| s.borrow().get(id))
}

#[ic_cdk::query]
fn get_message (id: u64) -> Result<Message, Error> {
    match _get_message(&id) {
        Some(message) => Ok(message),
        None => Err(Error::NotFound  {
            msg: format!("A message with the id: {} was not found", id),
        }),
    }
}

fn do_insert(message: &Message) {
    STORAGE.with(|s| s.borrow_mut().insert(message.id, message.clone()));
}

#[ic_cdk::update]
fn create_message (message: MessagePayload) -> Option<Message> {
    let message_id = ID_COUNTER.with( |c| {
        let current_value = *c.borrow().get();
        c.borrow_mut().set(current_value + 1)
    }).expect("Failed to increment the id counter");

    let message = Message{
        id: message_id,
        title: message.title,
        body: message.body,
        attachment_url: message.attachment_url,
        created_at: time(),
        updated_at: None,
    };

    do_insert(&message);
    Some(message)
}

#[ic_cdk::update]
fn update_message (id: u64, payload: MessagePayload) -> Result<Message, Error>{
    match STORAGE.with(|s| s.borrow().get(&id)) {
        Some(mut message) => {
            message.body = payload.body;
            message.title = payload.title;
            message.attachment_url = payload.attachment_url;
            message.updated_at = Some(time());
            do_insert(&message);
            Ok(message)
        },

        None => Err(Error::NotFound {
            msg: format!("Could not update message with id={}, message not be found", id),
        }),
    }
}

#[ic_cdk::update]
fn delete (id: u64) -> Result<Message, Error>{
    match STORAGE.with(|s| s.borrow_mut().remove(&id)) {
        Some(message) => Ok(message),
        None => Err(Error::NotFound {
            msg: format!("Could not delete message with id={}, message not found", id),
        }),
    }

}


// need this to generate candid
ic_cdk::export_candid!();
