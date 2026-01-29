use crate::app::{AppMessage, AppState};
use crate::common::AsyncValue;
use crate::domain::*;
use crate::port::network::{AddFriendError, FetchFriendListError};
use crate::ui::*;
use chrono::Utc;
use crossbeam_channel::Sender;
use eframe::egui;
use eframe::egui::TextBuffer;
use std::cell::RefCell;
use std::rc::Rc;

pub struct LobbyPage {
    // global
    app_state: Rc<RefCell<dyn AppState>>,
    message_tx: Sender<AppMessage>,
    // input
    chat_message_buf: String,
    new_friend_buf: String,
    new_group_buf: String,
    invite_buf: String,
    // display
    can_open_chat: bool,
    current_chat: Option<Conversation>,
    chat_history: Vec<String>,
}

impl LobbyPage {
    pub fn new(app_state: Rc<RefCell<dyn AppState>>, message_tx: Sender<AppMessage>) -> Self {
        Self {
            app_state,
            message_tx,
            chat_message_buf: String::new(),
            new_friend_buf: String::new(),
            new_group_buf: String::new(),
            invite_buf: String::new(),
            can_open_chat: true,
            current_chat: None,
            chat_history: Vec::new(),
        }
    }
}

impl Page for LobbyPage {
    #[cfg(debug_assertions)]
    fn ensure_state(&self) {
        debug_assert!(self.app_state.borrow().try_get_auth_tokens().is_some());
    }

    fn prepare_resource(&mut self) {
        self.app_state.borrow_mut().prepare_add_friend_state();
        self.app_state.borrow_mut().prepare_friend_list();
        self.app_state.borrow_mut().prepare_connection();
        self.app_state.borrow_mut().prepare_conversation();
    }

    fn drop_resource(&mut self) {
        self.app_state.borrow_mut().drop_add_friend_state();
        self.app_state.borrow_mut().drop_friend_list();
        self.app_state.borrow_mut().drop_connection();
        self.app_state.borrow_mut().drop_conversation();
    }

    fn init_message(&self) {
        let _ = self.message_tx.send(AppMessage::FriendListRequest);
        let _ = self.message_tx.send(AppMessage::EstablishConnectionRequest);
    }

    fn view(&mut self, ctx: &dyn ViewContext) -> Option<PageKind> {
        let ctx = downcast(ctx);

        let mut next_page = None;

        let size = ctx.input(|i| i.screen_rect().size());

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("Logout").clicked() {
                    // TODO: logout logic
                    next_page = Some(PageKind::Login);
                }
            });
        });

        egui::Window::new("Relationship")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.separator();
                ui.vertical_centered(|ui| {
                    if self.app_state.borrow().try_get_connection_state().is_none() {
                        match self.app_state.borrow().get_connection_request_state() {
                            AsyncValue::Pending => {
                                ui.spinner();
                                ui.label("Connecting...");
                            }
                            AsyncValue::Idle | AsyncValue::Ready(Err(_)) => {
                                if ui.button("Reconnect").clicked() {
                                    let _ = self
                                        .message_tx
                                        .send(AppMessage::EstablishConnectionRequest);
                                }
                            }
                            AsyncValue::Ready(Ok(_)) => {
                                unreachable!("connection state mismatch")
                            }
                        }
                    } else {
                        ui.label("Connected!");
                    }
                });

                ui.separator();

                // friend list
                ui.vertical_centered(|ui| {
                    ui.label("Friends");
                });
                ui.separator();
                match self.app_state.borrow().get_add_friend_state() {
                    AsyncValue::Pending => {
                        ui.spinner();
                    }
                    _ => {
                        widget::button_input(ui, "Add", &mut self.new_friend_buf, |buffer| {
                            let _ = self
                                .message_tx
                                .send(AppMessage::AddFriendRequest(buffer.take()));
                        });
                    }
                }
                match self.app_state.borrow().get_friend_list() {
                    AsyncValue::Idle => {
                        if ui.button("Fetch friend list").clicked() {
                            let _ = self.message_tx.send(AppMessage::FriendListRequest);
                        }
                    }
                    AsyncValue::Pending => {
                        ui.spinner();
                        ui.label("Retrieving friend list...");
                    }
                    AsyncValue::Ready(Ok(friend_list)) => {
                        if friend_list.is_empty() {
                            ui.label("No friend relationship.");
                        } else {
                            for friend in friend_list {
                                if ui.button(&friend.username).clicked() {
                                    let _ = self
                                        .message_tx
                                        .send(AppMessage::OpenConversation(friend.conversation_id));
                                    self.current_chat = Some(Conversation {
                                        id: friend.conversation_id,
                                        name: friend.username.clone(),
                                        kind: ConversationKind::Direct,
                                    })
                                }
                            }
                        }
                    }
                    AsyncValue::Ready(Err(error)) => {
                        if ui.button("Retry").clicked() {
                            let _ = self.message_tx.send(AppMessage::FriendListRequest);
                        }
                        ui.label("Fetch friend list failed.");
                        let _ = self.message_tx.send(AppMessage::FriendListRequest);
                    }
                }
                ui.separator();

                // group list
                ui.vertical_centered(|ui| {
                    ui.label("Groups");
                });
                ui.separator();
                widget::button_input(ui, "Create", &mut self.new_group_buf, |buffer| {
                    let _ = self.message_tx.send(AppMessage::CreateGroup(buffer.take()));
                });
                for i in 0..2 {
                    let name = format!("Group {}", i);
                    if ui.button(&name).clicked() {
                        self.current_chat = Some(Conversation {
                            id: ConversationId(uuid::Uuid::nil()),
                            name,
                            kind: ConversationKind::Group,
                        })
                    }
                }
                ui.separator();

                // recent conversation list
                ui.vertical_centered(|ui| {
                    ui.label("Recent Conversations");
                });
                ui.separator();
                for i in 0..2 {
                    ui.button(format!("Friend {}", i)).clicked();
                }
            });

        if let Some(conv) = &self.current_chat {
            egui::Window::new(&conv.name)
                .open(&mut self.can_open_chat)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, 0.0])
                .show(ctx, |ui| {
                    if matches!(conv.kind, ConversationKind::Group) {
                        widget::button_input(ui, "Invite", &mut self.invite_buf, |buffer| {
                            let _ = self
                                .message_tx
                                .send(AppMessage::AddFriendRequest(buffer.take()));
                        });
                        for i in 0..2 {
                            let name = format!("Friend {}", i);
                            ui.button(&name).clicked();
                        }
                        ui.separator();
                    }

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .max_height(size.y / 2.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            // for message in &self.chat_history {
                            //     ui.label(message);
                            // }
                            for message in self.app_state.borrow().get_conversation_history(conv.id)
                            {
                                match message {
                                    HistoryMessage::Concrete(c) => {
                                        ui.label(c.content);
                                    }
                                    HistoryMessage::Request(r) => {
                                        ui.label(format!("{}⏳", r.content));
                                    }
                                }
                            }
                        });

                    ui.separator();

                    widget::button_input(ui, "Send", &mut self.chat_message_buf, |buffer| {
                        let conversation_id = conv.id;
                        let message_id = MessageId(uuid::Uuid::new_v4());
                        let content = buffer.take();
                        let _ = self.message_tx.send(AppMessage::ChatMessageRequest(
                            ChatMessageInput {
                                conversation_id,
                                message_id,
                                content,
                                created_at: Utc::now(),
                            },
                        ));
                    });
                });

            if !self.can_open_chat {
                self.current_chat = None;
                self.can_open_chat = true;
            }
        }

        next_page
    }
}
