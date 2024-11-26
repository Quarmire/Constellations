use std::ops::Range;
use std::vec;
use cola::{Deletion, EncodedReplica, Replica, ReplicaId};
use serde::{Deserialize, Serialize};
use postcard;
use ulid::Ulid;
use crate::asset::{Asset, Holographable, Materializable};
use super::Block;

impl Asset for Text {
    fn id(&self) -> Ulid {
        todo!()
    }

    fn name(&self) {
        todo!()
    }

    fn tag(&self) {
        todo!()
    }

    fn untag(&self) {
        todo!()
    }

    fn flag(&self) {
        todo!()
    }

    fn unflag(&self) {
        todo!()
    }

    fn release(&self) {
        todo!()
    }

    fn derive() -> Self {
        todo!()
    }
    
    fn upload() {
        todo!()
    }
    
    fn download() -> Self {
        todo!()
    }
    
    fn fork() -> Self {
        todo!()
    }
}

impl Block for Text {}

impl Holographable for Text {
    fn update() {
        todo!()
    }
}

impl Materializable for Text {
    fn scan() {
        todo!()
    }
}

// Text content holds arbitrary text data that is able to be
// synchronized across spaceports.
// By default, all blocks follow the single-holder principle,
// but text can enable simultaneous editing with CRDTs.

pub struct Text {
    pub buffer: String,
    crdt: Replica,
    history: Vec<Edit>,
}

#[derive(Serialize, Deserialize)]
pub struct EncodedText {
    pub buffer: String,
    crdt: EncodedReplica,
    history: Vec<Edit>,
    assigned_id: u64,
}

impl Text {
    pub fn new<S: Into<String>>(text: S, replica_id: ReplicaId) -> Self {
        let buffer = text.into();
        let crdt = Replica::new(replica_id, buffer.len());
        let history: Vec<Edit> = vec![];
        Text { buffer, crdt, history }
    }

    fn fork(&self, new_replica_id: ReplicaId) -> Self {
        let crdt = self.crdt.fork(new_replica_id);
        Text { buffer: self.buffer.clone(), crdt , history: self.history.clone() }
    }

    pub fn encode(&self, assigned_id: u64) -> Vec<u8> {
        let encoded = self.crdt.encode();
        postcard::to_allocvec(&EncodedText{ buffer: self.buffer.clone(), crdt: encoded, history: self.history.clone(), assigned_id }).unwrap()
    }

    pub fn insert<S: Into<String>>(&mut self, insert_at: usize, text: S) -> Insertion {
        let text = text.into();
        self.buffer.insert_str(insert_at, &text);
        let insertion = self.crdt.inserted(insert_at, text.len());
        let edit = Insertion { text, crdt: insertion };
        self.history.insert(0, Edit::Inserted(edit.clone()));
        edit
    }

    pub fn delete(&mut self, range: Range<usize>) -> Deletion {
        self.buffer.replace_range(range.clone(), "");
        let edit = self.crdt.deleted(range);
        self.history.insert(0, Edit::Deleted(edit.clone()));
        edit
    }

    // TODO: make this into a gap buffer implementation
    pub fn integrate_insertion(&mut self, insertion: Insertion) {
        if let Some(offset) = self.crdt.integrate_insertion(&insertion.crdt) {
            self.buffer.insert_str(offset, &insertion.text); // O(n) operation!
        }
        self.history.insert(0, Edit::Inserted(insertion));
    }

    pub fn integrate_deletion(&mut self, deletion: Deletion) {
        let ranges = self.crdt.integrate_deletion(&deletion);
        for range in ranges.into_iter().rev() {
            self.buffer.replace_range(range, "");
        }
        self.history.insert(0, Edit::Deleted(deletion));
    }
}

impl From<EncodedText> for Text {
    fn from(value: EncodedText) -> Self {
        Text {
            buffer: value.buffer,
            crdt: Replica::decode(value.assigned_id, &value.crdt).unwrap(),
            history: value.history,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Insertion {
    text: String,
    crdt: cola::Insertion,
}

impl Insertion {
    fn encode(&self) -> Vec<u8> {
        postcard::to_allocvec(self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Edit {
    Inserted(Insertion),
    Deleted(Deletion),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ins_del() {
        let mut peer_1 = Text::new("Hello, world", 1);
        let mut peer_2 = peer_1.fork(2);

        let delete_comma = peer_1.delete(5..6);
        let insert_exclamation = peer_2.insert(12, "!");

        peer_1.integrate_insertion(insert_exclamation);
        peer_2.integrate_deletion(delete_comma);

        assert_eq!(peer_1.buffer, "Hello world!");
        assert_eq!(peer_2.buffer, "Hello world!");
    }

    #[test]
    fn ser_de() {
        let peer_1 = Text::new("Hello, world", 1);

        let encoded = peer_1.encode(2);
        let wire = encoded.as_slice();

        let peer_2_enc = postcard::from_bytes::<EncodedText>(wire).unwrap();
        let peer_2: Text = From::from(peer_2_enc);

        assert_eq!(peer_1.buffer, peer_2.buffer);
        assert_eq!(peer_2.crdt.id(), 2);
    }

    #[test]
    fn history() {
        let mut peer_1 = Text::new("Hello, world", 1);
        let mut peer_2 = peer_1.fork(2);
        let mut peer_3 = peer_1.fork(3);
        let mut peer_4 = peer_1.fork(4);

        let delete_comma = peer_1.delete(5..6);
        let insert_exclamation = peer_2.insert(12, "!");

        peer_1.integrate_insertion(insert_exclamation);
        peer_2.integrate_deletion(delete_comma);

        let history_1 = peer_1.history.clone();
        let history_2 = peer_2.history.clone();

        println!("{:?}", history_1);
        println!("{:?}", history_2);

        for edit in history_1 {
            match edit {
                Edit::Inserted(insertion) => {
                    peer_3.integrate_insertion(insertion);
                }
                Edit::Deleted(deletion) => {
                    peer_3.integrate_deletion(deletion);
                }
            }
        }

        for edit in history_2 {
            match edit {
                Edit::Inserted(insertion) => {
                    peer_4.integrate_insertion(insertion);
                }
                Edit::Deleted(deletion) => {
                    peer_4.integrate_deletion(deletion);
                }
            }
        }

        assert_eq!(peer_3.buffer, "Hello world!");
        assert_eq!(peer_4.buffer, "Hello world!");
    }
}