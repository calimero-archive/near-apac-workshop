use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;
use near_sdk::store::{UnorderedMap, UnorderedSet};
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault, PublicKey};

use std::collections::HashMap;
use std::fmt::Write;

type MessageId = String;

const ACTIVE_MS_THRESHOLD: u64 = 30 * 1000;

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Hash,
)]
#[serde(crate = "near_sdk::serde")]
pub struct Channel {
    pub name: String,
}

#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
#[serde(crate = "near_sdk::serde")]
pub struct ChannelWithId {
    pub name: String,
    pub id: String,
}

#[derive(
    BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
#[serde(crate = "near_sdk::serde")]
pub struct UserInfo {
    pub id: AccountId,
    pub active: bool,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone,
)]
#[serde(crate = "near_sdk::serde")]
pub struct Message {
    pub timestamp: u64,
    pub sender: AccountId,
    pub id: MessageId,
    pub text: String,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MessageWithReactions {
    pub id: MessageId,
    pub text: String,
    pub timestamp: u64,
    pub sender: AccountId,
    pub reactions: Option<HashMap<MessageId, Vec<AccountId>>>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MessageWithReactionsAndThread {
    pub id: MessageId,
    pub text: String,
    pub timestamp: u64,
    pub sender: AccountId,
    pub reactions: Option<HashMap<MessageId, Vec<AccountId>>>,
    pub thread: Vec<MessageWithReactions>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct UnreadMessage {
    pub count: usize,
    #[serde(rename = "lastSeen")]
    pub last_seen: Option<MessageId>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct UnreadMessageInfo {
    pub channels: HashMap<String, UnreadMessage>,
    pub chats: HashMap<AccountId, UnreadMessage>,
    pub threads: HashMap<MessageId, UnreadMessage>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ChannelMetadata {
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "createdBy")]
    pub created_by: AccountId,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ChannelInfo {
    pub messages: Vec<Message>,
    pub is_public: bool,
    pub meta: ChannelMetadata,
    pub last_read: UnorderedMap<AccountId, MessageId>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Curb {
    name: String,

    created_at: u64,
    members: UnorderedMap<AccountId, u64>,
    member_keys: UnorderedMap<AccountId, PublicKey>,

    channels: UnorderedMap<Channel, ChannelInfo>,
    channel_members: UnorderedMap<Channel, UnorderedSet<AccountId>>,
    member_channels: UnorderedMap<AccountId, UnorderedSet<Channel>>,

    chats: UnorderedMap<(AccountId, AccountId), ChannelInfo>,

    threads: UnorderedMap<MessageId, Vec<Message>>,

    reactions: UnorderedMap<MessageId, UnorderedMap<String, UnorderedSet<AccountId>>>,
}

#[near_bindgen]
impl Curb {
    #[init(ignore_state)]
    pub fn new(name: String) -> Self {
        Self {
            name,
            created_at: env::block_timestamp_ms(),
            members: UnorderedMap::new(b"m".to_vec()),
            member_keys: UnorderedMap::new(b"k".to_vec()),
            channels: UnorderedMap::new(b"n".to_vec()),
            channel_members: UnorderedMap::new(b"c".to_vec()),
            member_channels: UnorderedMap::new(b"e".to_vec()),
            chats: UnorderedMap::new(b"t".to_vec()),
            threads: UnorderedMap::new(b"h".to_vec()),
            reactions: UnorderedMap::new(b"r".to_vec()),
        }
    }

    fn default_channel() -> Channel {
        Channel {
            name: "general".to_string(),
        }
    }

    fn register_activity(&mut self) {
        self.members
            .insert(env::predecessor_account_id(), env::block_timestamp_ms());
        // TODO support multiple keys
        self.member_keys
            .insert(env::predecessor_account_id(), env::signer_account_pk());
    }

    #[payable]
    pub fn join(&mut self) {
        // TODO handle storage payments
        require!(
            !self.members.contains_key(&env::predecessor_account_id()),
            "Already a member"
        );
        if self.members.is_empty() {
            self.internal_create_group(Curb::default_channel(), false);
        }
        self.register_activity();
        self.member_channels.insert(
            env::predecessor_account_id(),
            UnorderedSet::new(env::predecessor_account_id().as_bytes()),
        );
        self.join_group(Curb::default_channel());
    }

    #[payable]
    pub fn ping(&mut self) {
        self.register_activity();
    }

    fn order_accounts(account: AccountId, other_account: AccountId) -> (AccountId, AccountId) {
        let account1 = if account.as_str() < other_account.as_str() {
            account.clone()
        } else {
            other_account.clone()
        };

        let account2 = if account1.as_str() == other_account.as_str() {
            account
        } else {
            other_account
        };

        (account1, account2)
    }

    #[payable]
    pub fn create_group(&mut self, group: Channel) {
        // TODO handle storage payments
        self.internal_create_group(group, true);
        self.register_activity();
    }

    fn internal_create_group(&mut self, group: Channel, membership_required: bool) {
        require!(group.name.len() > 0, "Group name too short!");
        require!(!self.channels.contains_key(&group), "Group already exists");
        require!(
            !membership_required || self.members.contains_key(&env::predecessor_account_id()),
            "Not a member"
        );
        self.channels.insert(
            group.clone(),
            ChannelInfo {
                messages: vec![],
                is_public: true,
                meta: ChannelMetadata {
                    created_at: env::block_timestamp_ms(),
                    created_by: env::predecessor_account_id(),
                },
                last_read: UnorderedMap::new(env::sha256(group.name.as_bytes())),
            },
        );
        self.channel_members
            .insert(group.clone(), UnorderedSet::new(group.name.as_bytes()));
        if membership_required {
            self.join_group(group);
        }
    }

    #[payable]
    pub fn join_group(&mut self, group: Channel) {
        // TODO handle storage payments
        require!(self.channels.contains_key(&group), "Group does not exist");
        require!(
            self.members.contains_key(&env::predecessor_account_id()),
            "Not a member"
        );
        self.channel_members
            .get_mut(&group)
            .unwrap()
            .insert(env::predecessor_account_id());
        self.member_channels
            .get_mut(&env::predecessor_account_id())
            .unwrap()
            .insert(group.clone());
        self.register_activity();

        env::value_return(&serde_json::to_vec(&group).unwrap());
    }

    #[payable]
    pub fn leave_group(&mut self, group: Channel) {
        // TODO handle storage payments
        require!(self.channels.contains_key(&group), "Group does not exist");
        require!(
            self.members.contains_key(&env::predecessor_account_id()),
            "Not a member"
        );
        self.channel_members
            .get_mut(&group)
            .unwrap()
            .remove(&env::predecessor_account_id());
        self.member_channels
            .get_mut(&env::predecessor_account_id())
            .unwrap()
            .remove(&group);

        if self.channel_members.get(&group).unwrap().is_empty() && group != Curb::default_channel()
        {
            self.channel_members.remove(&group);
            self.channels.remove(&group);
        }
        self.register_activity();

        env::value_return(&serde_json::to_vec(&group).unwrap());
    }

    #[payable]
    pub fn group_invite(&mut self, group: Channel, account: AccountId) {
        // TODO handle storage payments
        require!(self.channels.contains_key(&group), "Group does not exist");
        require!(self.members.contains_key(&account), "Not a member");
        self.channel_members
            .get_mut(&group)
            .unwrap()
            .insert(account.clone());
        self.member_channels
            .get_mut(&account)
            .unwrap()
            .insert(group.clone());
        self.register_activity();

        env::value_return(&serde_json::to_vec(&group).unwrap());
    }

    fn get_message_id(
        account: &AccountId,
        other_account: &Option<AccountId>,
        group: &Option<Channel>,
        message: &String,
        timestamp: u64,
    ) -> MessageId {
        let target_bytes: &[u8];
        if let Some(acc) = other_account {
            target_bytes = acc.as_bytes();
        } else {
            target_bytes = group.as_ref().unwrap().name.as_bytes();
        }

        let bytes: &[u8] = &env::sha256(
            &[
                target_bytes,
                account.as_bytes(),
                message.as_bytes(),
                &timestamp.to_be_bytes(),
            ]
            .concat(),
        );

        let mut s = MessageId::with_capacity(bytes.len() * 2);
        for &b in bytes {
            write!(&mut s, "{:02x}", b).unwrap();
        }
        s
    }

    fn find_message_pos(messages: &Vec<Message>, message_id: &MessageId) -> usize {
        for (i, m) in messages.iter().enumerate().rev() {
            if &m.id == message_id {
                return i;
            }
        }

        0
    }

    fn find_last_seen_pos(&self, channel_info: &ChannelInfo, account: &AccountId) -> usize {
        match channel_info.last_read.get(&account) {
            Some(message_id) => Curb::find_message_pos(&channel_info.messages, message_id) + 1,
            _ => 0,
        }
    }

    #[payable]
    pub fn send_message(
        &mut self,
        account: Option<AccountId>,
        group: Option<Channel>,
        message: String,
        timestamp: u64,
        parent_message: Option<MessageId>,
    ) {
        // TODO handle storage payments
        require!(
            self.members.contains_key(&env::predecessor_account_id()),
            "Not a member"
        );
        self.register_activity();
        let message_id = Curb::get_message_id(
            &env::predecessor_account_id(),
            &account,
            &group,
            &message,
            timestamp,
        );
        let message = Message {
            id: message_id.clone(),
            text: message,
            sender: env::predecessor_account_id(),
            timestamp: timestamp,
        };
        if let Some(other) = account {
            require!(
                self.members.contains_key(&other),
                "Other account is not a member"
            );

            let key = Curb::order_accounts(env::predecessor_account_id(), other.clone());

            if let Some(parent_id) = parent_message {
                let container = self.threads.entry(parent_id).or_insert(vec![]);
                let pos = container.binary_search(&message).unwrap_or_else(|e| e);
                container.insert(pos, message);
            } else {
                let chat = self.chats.entry(key.clone()).or_insert(ChannelInfo {
                    messages: vec![],
                    is_public: false,
                    meta: ChannelMetadata {
                        created_at: env::block_timestamp_ms(),
                        created_by: env::predecessor_account_id(),
                    },
                    last_read: UnorderedMap::new(format!("{}#{}", key.0, key.1).as_bytes()),
                });
                let messages = &mut chat.messages;

                let pos = messages.binary_search(&message).unwrap_or_else(|e| e);
                messages.insert(pos, message);

                self.read_message(Some(other.clone()), None, message_id);
            }

            env::value_return(&serde_json::to_vec(&other).unwrap());
        } else if let Some(channel) = group {
            require!(self.channels.contains_key(&channel), "Group does not exist");
            let is_member = match self.channel_members.get(&channel) {
                Some(cm) => cm.contains(&env::predecessor_account_id()),
                None => false,
            };
            require!(is_member, "Not a group member");
            if let Some(parent_id) = parent_message {
                let container = self.threads.entry(parent_id).or_insert(vec![]);
                let pos = container.binary_search(&message).unwrap_or_else(|e| e);
                container.insert(pos, message);
            } else {
                let messages = &mut self.channels.get_mut(&channel).unwrap().messages;

                let pos = messages.binary_search(&message).unwrap_or_else(|e| e);
                messages.insert(pos, message);

                self.read_message(None, Some(channel.clone()), message_id);
            }

            env::value_return(&serde_json::to_vec(&channel).unwrap());
        } else {
            panic!("Either account or group need to be provided");
        }
    }

    #[payable]
    pub fn read_message(
        &mut self,
        account: Option<AccountId>,
        group: Option<Channel>,
        message_id: MessageId,
    ) {
        if let Some(other) = account {
            let key = Curb::order_accounts(env::predecessor_account_id(), other.clone());
            // TODO handle possibility that your message was put before last message currently seen.
            self.chats
                .get_mut(&key)
                .unwrap()
                .last_read
                .insert(env::predecessor_account_id(), message_id);
        } else if let Some(channel) = group {
            // TODO handle possibility that your message was put before last message currently seen.
            self.channels
                .get_mut(&channel)
                .unwrap()
                .last_read
                .insert(env::predecessor_account_id(), message_id);
        } else {
            panic!("Either account or group need to be provided");
        }
    }

    #[payable]
    pub fn toggle_reaction(&mut self, message_id: MessageId, reaction: String) {
        // TODO handle storage payments
        let reactions = self
            .reactions
            .entry(message_id.clone())
            .or_insert(UnorderedMap::new(message_id.as_bytes()));
        let tracker = &mut reactions
            .entry(reaction.clone())
            .or_insert(UnorderedSet::new(
                format!("{} {}", message_id, reaction).as_bytes(),
            ));
        if tracker.contains(&env::predecessor_account_id()) {
            tracker.remove(&env::predecessor_account_id());
        } else {
            tracker.insert(env::predecessor_account_id());
        }
        self.register_activity();
    }

    pub fn unread_messages(&self, account: AccountId) -> UnreadMessageInfo {
        let mut unread_info = UnreadMessageInfo {
            channels: HashMap::new(),
            chats: HashMap::new(),
            threads: HashMap::new(),
        };
        for (channel, info) in self.channels.iter() {
            let unread = info.messages.len() - self.find_last_seen_pos(&info, &account);
            unread_info.channels.insert(
                channel.name.clone(),
                UnreadMessage {
                    count: unread,
                    last_seen: info.last_read.get(&account).cloned(),
                },
            );
        }

        for ((account1, account2), info) in self.chats.iter() {
            if account1 != &account && account2 != &account {
                continue;
            }
            let other_account = if account1 == &account {
                account2
            } else {
                account1
            };

            let unread = info.messages.len() - self.find_last_seen_pos(&info, &account);

            unread_info.chats.insert(
                other_account.clone(),
                UnreadMessage {
                    count: unread,
                    last_seen: info.last_read.get(&account).cloned(),
                },
            );
        }

        unread_info
    }

    fn add_reactions_to_message(&self, message: Message) -> MessageWithReactions {
        let mut message_with_reactions = MessageWithReactions {
            id: message.id.clone(),
            text: message.text,
            timestamp: message.timestamp,
            sender: message.sender,
            reactions: None,
        };

        let mut reactions: HashMap<String, Vec<AccountId>> = HashMap::new();

        if let Some(r) = self.reactions.get(&message.id) {
            for (k, v) in r.iter() {
                reactions.insert(k.clone(), v.iter().map(|a| a.clone()).collect());
            }
            message_with_reactions.reactions = Some(reactions);
        }

        message_with_reactions
    }

    fn add_thread_to_message(
        &self,
        message: MessageWithReactions,
    ) -> MessageWithReactionsAndThread {
        let empty_thread: Vec<Message> = vec![];
        MessageWithReactionsAndThread {
            id: message.id.clone(),
            text: message.text,
            timestamp: message.timestamp,
            sender: message.sender,
            reactions: message.reactions,
            thread: self
                .threads
                .get(&message.id)
                .unwrap_or(&empty_thread)
                .into_iter()
                .map(|m| self.add_reactions_to_message(m.clone()))
                .collect(),
        }
    }

    pub fn get_messages(
        &self,
        accounts: Option<(AccountId, AccountId)>,
        group: Option<Channel>,
        offset: Option<usize>,
        length: Option<usize>,
    ) -> Vec<MessageWithReactionsAndThread> {
        let start = if let Some(pos) = offset { pos } else { 0 };
        if let Some((account1, account2)) = accounts {
            let key = Curb::order_accounts(account1, account2);

            match self.chats.get(&key) {
                Some(e) => {
                    let end = if let Some(len) = length {
                        std::cmp::min(len + start, e.messages.len())
                    } else {
                        e.messages.len() - start
                    };

                    e.messages[start..end]
                        .into_iter()
                        .map(|m| self.add_reactions_to_message(m.clone()))
                        .map(|m| self.add_thread_to_message(m))
                        .collect()
                }
                None => vec![],
            }
        } else if let Some(channel) = group {
            match self.channels.get(&channel) {
                Some(e) => {
                    let end = if let Some(len) = length {
                        std::cmp::min(len + start, e.messages.len())
                    } else {
                        e.messages.len() - start
                    };
                    e.messages[start..end]
                        .into_iter()
                        .map(|m| self.add_reactions_to_message(m.clone()))
                        .map(|m| self.add_thread_to_message(m))
                        .collect()
                }
                None => vec![],
            }
        } else {
            panic!("Either account or group need to be provided");
        }
    }

    pub fn get_members(&self, group: Option<Channel>) -> Vec<UserInfo> {
        if let Some(group) = group {
            match self.channel_members.get(&group) {
                Some(cm) => cm
                    .iter()
                    .map(|m| UserInfo {
                        id: m.clone(),
                        active: self.is_active(*self.members.get(&m).unwrap()),
                    })
                    .collect(),
                None => vec![],
            }
        } else {
            self.members
                .iter()
                .map(|(m, timestamp)| UserInfo {
                    id: m.clone(),
                    active: self.is_active(*timestamp),
                })
                .collect()
        }
    }

    fn is_active(&self, timestamp: u64) -> bool {
        env::block_timestamp_ms() - timestamp < ACTIVE_MS_THRESHOLD
    }

    pub fn get_groups(&self, account: Option<AccountId>) -> Vec<&Channel> {
        if let Some(account) = account {
            match self.member_channels.get(&account) {
                Some(mc) => mc.iter().collect(),
                None => vec![],
            }
        } else {
            self.channels.keys().collect()
        }
    }

    pub fn get_keys(&self, account: AccountId) -> Vec<PublicKey> {
        match self.member_keys.get(&account) {
            Some(key) => vec![key.clone()],
            None => vec![],
        }
    }

    pub fn channel_info(&self, group: Channel) -> Option<&ChannelMetadata> {
        self.channels.get(&group).map(|c| &c.meta)
    }

    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }
}
