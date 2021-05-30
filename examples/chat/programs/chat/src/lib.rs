//! A simple chat program using a ring buffer to store messages.

use anchor_lang::prelude::*;

#[program]
pub mod chat {
    use super::*;

    pub fn create_user(ctx: Context<CreateUser>, name: String) -> Result<()> {
        ctx.accounts.user.name = name;
        ctx.accounts.user.authority = *ctx.accounts.authority.key;
        Ok(())
    }
    pub fn create_chat_room(ctx: Context<CreateChatRoom>, name: String) -> Result<()> {
        let given_name = name.as_bytes();
        let mut name = [0u8; 280];
        name[..given_name.len()].copy_from_slice(given_name);
        let mut chat = ctx.accounts.chat_room.load_init()?;
        chat.name = name;
        Ok(())
    }
    pub fn send_message(ctx: Context<SendMessage>, msg: String) -> Result<()> {
        let mut chat = ctx.accounts.chat_room.load_mut()?;
        chat.append({
            let src = msg.as_bytes();
            let mut data = [0u8; 280];
            data[..src.len()].copy_from_slice(src);
            Message {
                from: *ctx.accounts.user.to_account_info().key,
                data,
            }
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateUser<'info> {
    #[account(init, associated = authority, space = 312)]
    user: ProgramAccount<'info, User>,
    #[account(signer)]
    authority: AccountInfo<'info>,
    rent: Sysvar<'info, Rent>,
    system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateChatRoom<'info> {
    #[account(init)]
    chat_room: Loader<'info, ChatRoom>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SendMessage<'info> {
    #[account(associated = authority, has_one = authority)]
    user: ProgramAccount<'info, User>,
    #[account(signer)]
    authority: AccountInfo<'info>,
    #[account(mut)]
    chat_room: Loader<'info, ChatRoom>,
}

#[associated]
pub struct User {
    name: String,
    authority: Pubkey,
}

#[account(zero_copy)]
pub struct ChatRoom {
    head: u64,
    tail: u64,
    name: [u8; 280],            // Human readable name (char bytes).
    messages: [Message; 33607], // Leaves the account at 10,485,680 bytes.
}

impl ChatRoom {
    fn append(&mut self, msg: Message) {
        self.messages[ChatRoom::index_of(self.head)] = msg;
        if ChatRoom::index_of(self.head + 1) == ChatRoom::index_of(self.tail) {
            self.tail += 1;
        }
        self.head += 1;
    }
    fn index_of(counter: u64) -> usize {
        std::convert::TryInto::try_into(counter % 33607).unwrap()
    }
}

#[zero_copy]
pub struct Message {
    pub from: Pubkey,
    pub data: [u8; 280],
}

#[error]
pub enum ErrorCode {
    Unknown,
}
