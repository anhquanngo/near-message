use email::*;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::{
    assert_one_yocto, env, json_types::U128, near_bindgen, AccountId, BorshStorageKey,
    PanicOnDefault,
};
use storage_impl::*;

mod email;
mod storage_impl;
pub type EmailID = u128;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Sender,
    Receiver,
    Email,
    SenderMail { email_id: EmailID },
    ReceiverMail { email_id: EmailID },
    Account,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    senders: LookupMap<AccountId, UnorderedSet<EmailID>>,
    receivers: LookupMap<AccountId, UnorderedSet<EmailID>>,
    emails: UnorderedMap<EmailID, Email>,
    email_count: u128,
    accounts: LookupMap<AccountId, VAccount>,
    donation_contract_account: Option<AccountId>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            senders: LookupMap::new(StorageKeys::Sender),
            receivers: LookupMap::new(StorageKeys::Receiver),
            emails: UnorderedMap::new(StorageKeys::Email),
            email_count: 0,
            accounts: LookupMap::new(StorageKeys::Account),
            donation_contract_account: None,
        }
    }

    pub fn add_donation_contract_account(&mut self, account: AccountId) {
        self.donation_contract_account = Some(account);
    }

    #[payable]
    pub fn send_mail(
        &mut self,
        receiver: AccountId,
        title: String,
        content: String,
        fee: Option<U128>,
    ) {
        assert_one_yocto();
        let sender = env::predecessor_account_id();
        assert!(
            self.accounts.contains_key(&sender),
            "Account not registered"
        );
        assert!(self.can_send_mail(sender.clone()), "Not deposit enough");

        let mut vaccount = self.accounts.get(&sender).unwrap();
        vaccount.used += STORAGE_PER_MAIL * env::storage_byte_cost();
        self.accounts.insert(&sender, &vaccount);
        let current_count = self.email_count;
        self.email_count = self.email_count + 1;
        let timestamp = env::block_timestamp();

        if Some(sender) == self.donation_contract_account {
            assert!(fee.is_none(), "Fee must be none");
        }

        let email = Email {
            title,
            content,
            timestamp,
            fee,
        };
        self.emails.insert(&current_count, &email);
        if let Some(mut sender_vec) = self.senders.get(&sender) {
            sender_vec.insert(&current_count);
            self.senders.insert(&sender, &sender_vec);
        } else {
            let mut sender_vec_new = UnorderedSet::new(StorageKeys::SenderMail {
                email_id: current_count,
            });
            sender_vec_new.insert(&current_count);
            self.senders.insert(&sender, &sender_vec_new);
        }

        if let Some(mut receiver_vec) = self.receivers.get(&receiver) {
            receiver_vec.insert(&current_count);
            self.receivers.insert(&receiver, &receiver_vec);
        } else {
            let mut receiver_vec_new = UnorderedSet::new(StorageKeys::ReceiverMail {
                email_id: current_count,
            });
            receiver_vec_new.insert(&current_count);
            self.receivers.insert(&receiver, &receiver_vec_new);
        }
    }

    pub fn get_email(&self, email_id: U128) -> Email {
        let real_email_id: EmailID = email_id.0;
        self.emails.get(&real_email_id).unwrap()
    }

    pub fn delete_mail(&mut self, email_id: U128) {
        let real_email_id: EmailID = email_id.0;
        let sender = env::predecessor_account_id();
        assert!(
            !self.senders.get(&sender).unwrap().contains(&real_email_id),
            "Caller is not sender"
        );
        self.emails.remove(&real_email_id);
    }

    pub fn mail_exist(&self) -> u64 {
        self.emails.keys_as_vector().len()
    }

    pub fn get_mail_receive(&self, receiver: AccountId) -> Vec<Email> {
        let mut email_vec: Vec<Email> = Vec::new();
        if let Some(receiver_vec) = self.receivers.get(&receiver) {
            for index in receiver_vec.iter() {
                let mail = self.emails.get(&index).unwrap();
                email_vec.push(mail);
            }
        }
        return email_vec;
    }

    pub fn get_mail_send(&self, sender: AccountId) -> Vec<Email> {
        let mut email_vec: Vec<Email> = Vec::new();
        if let Some(sender_vec) = self.senders.get(&sender) {
            for index in sender_vec.iter() {
                let mail = self.emails.get(&index).unwrap();
                email_vec.push(mail);
            }
        }
        return email_vec;
    }

    pub fn get_mail_receive_num(&self, receiver: AccountId) -> u64 {
        if let Some(receiver_vec) = self.receivers.get(&receiver) {
            return receiver_vec.len();
        }
        0
    }

    pub fn get_mail_send_num(&self, sender: AccountId) -> u64 {
        if let Some(sender_vec) = self.senders.get(&sender) {
            return sender_vec.len();
        }
        0
    }

    pub fn mail_delete(&self) -> U128 {
        let mail_exist: u128 = self.emails.keys_as_vector().len().into();
        U128(self.email_count - mail_exist)
    }
}

impl Contract {
    pub fn can_send_mail(&self, account_id: AccountId) -> bool {
        let available_storage = self
            .storage_balance_of(account_id.clone())
            .unwrap()
            .available
            .0;

        if let Some(sender) = self.senders.get(&account_id) {
            let mail_len: u128 = (sender.len() + 1).into();
            return available_storage > (mail_len * STORAGE_PER_MAIL * env::storage_byte_cost());
        }
        return available_storage > (STORAGE_PER_MAIL * env::storage_byte_cost());
    }
}
