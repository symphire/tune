use chrono::Utc;
use crossbeam_channel::{Receiver, Sender};
use nanoid::nanoid;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, info, info_span, trace};
use tracing_subscriber::EnvFilter;
use tune::app::{AppMessage, AppState};
use tune::domain::{
    CaptchaId, ChatMessageInput, ConversationId, LoginInput, MessageId, SignupInput,
};
use tune::infra::network::RealNetwork;
use tune::port::network::{Network, StreamMessage};
use tune::state::RealAppState;

struct Client {
    pub username: String,
    pub password: String,
    pub app_state: Rc<RefCell<dyn AppState>>,
    pub message_tx: Sender<AppMessage>,
    pub message_rx: Receiver<AppMessage>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("tune=trace,app_state_demo=trace"))
        .init();

    let alphabet: [char; 16] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let run_id = nanoid!(10, &alphabet);

    const USERNAME_PREFIX: &str = "testuser";
    const PASSWORD: &str = "testpass";
    const CAPTCHA: &str = "123456";

    // username, password, state, tx, rx
    let mut clients: Vec<Client> = Vec::new();

    for i in 0..2 {
        let span = info_span!("create clients", iteration = i);
        let _guard = span.enter();

        let username = format!("{}{}_{}", USERNAME_PREFIX, i, run_id);
        let password = PASSWORD.to_owned();
        let captcha = CAPTCHA.to_owned();

        info!("initialize app state");
        let network: Rc<RefCell<dyn Network>> =
            Rc::new(RefCell::new(RealNetwork::try_new().unwrap()));

        let (message_tx, message_rx) = crossbeam_channel::bounded(2048);
        let app_state: Rc<RefCell<dyn AppState>> =
            Rc::new(RefCell::new(RealAppState::new(message_tx.clone(), network)));

        clients.push(Client {
            username,
            password,
            app_state,
            message_tx,
            message_rx,
        });
        let client = clients.last().unwrap();

        info!("signup");
        let captcha_key = client.app_state.borrow_mut().prepare_captcha();
        client.app_state.borrow_mut().prepare_signup_state();
        client
            .app_state
            .borrow_mut()
            .update(AppMessage::CaptchaRequest(captcha_key));
        if let Ok(app_message) = client.message_rx.recv() {
            trace!("captcha event on signup: {:?}", app_message);
            client.app_state.borrow_mut().update(app_message);
        }

        client
            .app_state
            .borrow_mut()
            .update(AppMessage::SignupRequest(SignupInput {
                username: client.username.clone(),
                password: client.password.clone(),
                captcha_id: CaptchaId(uuid::Uuid::nil()),
                captcha_answer: captcha.clone(),
            }));
        if let Ok(app_message) = client.message_rx.recv() {
            trace!("signup event: {:?}", app_message);
            client.app_state.borrow_mut().update(app_message);
        }

        client.app_state.borrow_mut().drop_captcha(captcha_key);
        client.app_state.borrow_mut().drop_signup_state();

        info!("login");
        let captcha_key = client.app_state.borrow_mut().prepare_captcha();
        client.app_state.borrow_mut().prepare_login_state();
        client
            .app_state
            .borrow_mut()
            .update(AppMessage::CaptchaRequest(captcha_key));
        if let Ok(app_message) = client.message_rx.recv() {
            trace!("captcha event on login: {:?}", app_message);
            client.app_state.borrow_mut().update(app_message);
        }

        client
            .app_state
            .borrow_mut()
            .update(AppMessage::LoginRequest(LoginInput {
                username: client.username.clone(),
                password: client.password.clone(),
                captcha_id: CaptchaId(uuid::Uuid::nil()),
                captcha_answer: captcha.clone(),
            }));
        if let Ok(app_message) = client.message_rx.recv() {
            trace!("login event: {:?}", app_message);
            client.app_state.borrow_mut().update(app_message);
        }

        client.app_state.borrow_mut().drop_captcha(captcha_key);
        client.app_state.borrow_mut().drop_login_state();

        info!("lobby");
        debug_assert!(client.app_state.borrow().try_get_auth_tokens().is_some());
        client.app_state.borrow_mut().prepare_add_friend_state();
        client.app_state.borrow_mut().prepare_friend_list();
        client.app_state.borrow_mut().prepare_connection();
        client.app_state.borrow_mut().prepare_conversation();

        client
            .app_state
            .borrow_mut()
            .update(AppMessage::FriendListRequest);
        if let Ok(app_message) = client.message_rx.recv() {
            trace!("friend list event: {:?}", app_message);
            client.app_state.borrow_mut().update(app_message);
        }
        client
            .app_state
            .borrow_mut()
            .update(AppMessage::EstablishConnectionRequest);
        if let Ok(app_message) = client.message_rx.recv() {
            trace!("establish connection event: {:?}", app_message);
            client.app_state.borrow_mut().update(app_message);
        }
    }

    info!("add friend");
    debug_assert!(clients[0].message_rx.is_empty());
    debug_assert!(clients[1].message_rx.is_empty());

    let mut conv: Option<ConversationId> = None;
    clients[0]
        .app_state
        .borrow_mut()
        .update(AppMessage::AddFriendRequest(clients[1].username.clone()));
    if let Ok(app_message) = clients[0].message_rx.recv() {
        if let AppMessage::AddFriendEvent(e) = &app_message {
            trace!("add friend event: {:?}", e.body.is_ok());
            conv = Some(e.body.as_ref().unwrap().clone());
        }
        clients[0].app_state.borrow_mut().update(app_message);
    }
    if let Ok(app_message) = clients[0].message_rx.recv() {
        trace!("internal friend list request: {:?}", app_message);
        clients[0].app_state.borrow_mut().update(app_message);
    }
    if let Ok(app_message) = clients[0].message_rx.recv() {
        trace!(
            "friend list received (left={}): {:?}",
            clients[0].message_rx.len(),
            app_message
        );
        clients[0].app_state.borrow_mut().update(app_message);
    }
    if let Ok(app_message) = clients[1].message_rx.recv() {
        trace!("friendship received: {:?}", app_message);
        clients[1].app_state.borrow_mut().update(app_message);
    }
    if let Ok(app_message) = clients[1].message_rx.recv() {
        trace!("internal friend list request: {:?}", app_message);
        clients[1].app_state.borrow_mut().update(app_message);
    }

    info!("send chat message");
    debug_assert!(clients[0].message_rx.is_empty());
    debug_assert!(clients[1].message_rx.is_empty());

    clients[0]
        .app_state
        .borrow_mut()
        .update(AppMessage::OpenConversation(conv.unwrap()));
    if let Ok(app_message) = clients[0].message_rx.recv() {
        trace!("history received: {:?}", app_message);
        clients[0].app_state.borrow_mut().update(app_message);
    }

    clients[0]
        .app_state
        .borrow_mut()
        .update(AppMessage::ChatMessageRequest(ChatMessageInput {
            conversation_id: conv.unwrap(),
            message_id: MessageId(uuid::Uuid::new_v4()),
            content: "Hello!".to_string(),
            created_at: Utc::now(),
        }));
    if let Ok(app_message) = clients[0].message_rx.recv() {
        trace!("chat message sent: {:?}", app_message);
        clients[0].app_state.borrow_mut().update(app_message);
    }

    clients[0]
        .app_state
        .borrow_mut()
        .update(AppMessage::ChatMessageRequest(ChatMessageInput {
            conversation_id: conv.unwrap(),
            message_id: MessageId(uuid::Uuid::new_v4()),
            content: "How are you!".to_string(),
            created_at: Utc::now(),
        }));
    if let Ok(app_message) = clients[0].message_rx.recv() {
        trace!("chat message sent: {:?}", app_message);
        clients[0].app_state.borrow_mut().update(app_message);
    }
    debug!(
        "history: {:?}",
        clients[0]
            .app_state
            .borrow()
            .get_conversation_history(conv.unwrap())
    );

    println!("{}", clients[1].message_tx.len());
    if let Ok(app_message) = clients[1].message_rx.recv() {
        trace!("chat message received: {:?}", app_message);
        clients[1].app_state.borrow_mut().update(app_message);
    }
    if let Ok(app_message) = clients[1].message_rx.recv() {
        trace!("chat message received: {:?}", app_message);
        clients[1].app_state.borrow_mut().update(app_message);
    }

    clients[1]
        .app_state
        .borrow_mut()
        .update(AppMessage::OpenConversation(conv.unwrap()));
    if let Ok(app_message) = clients[1].message_rx.recv() {
        trace!("history received: {:?}", app_message);
        clients[1].app_state.borrow_mut().update(app_message);
    }
    debug!(
        "history: {:?}",
        clients[1]
            .app_state
            .borrow()
            .get_conversation_history(conv.unwrap())
    );

    clients[1]
        .app_state
        .borrow_mut()
        .update(AppMessage::ChatMessageRequest(ChatMessageInput {
            conversation_id: conv.unwrap(),
            message_id: MessageId(uuid::Uuid::new_v4()),
            content: "Fine. Thank you!".to_string(),
            created_at: Utc::now(),
        }));
    debug!(
        "history: {:?}",
        clients[1]
            .app_state
            .borrow()
            .get_conversation_history(conv.unwrap())
    );

    if let Ok(app_message) = clients[1].message_rx.recv() {
        trace!("chat message sent: {:?}", app_message);
        clients[1].app_state.borrow_mut().update(app_message);
    }
    debug!(
        "history: {:?}",
        clients[1]
            .app_state
            .borrow()
            .get_conversation_history(conv.unwrap())
    );

    if let Ok(app_message) = clients[0].message_rx.recv() {
        trace!("chat message received: {:?}", app_message);
        clients[0].app_state.borrow_mut().update(app_message);
    }
}
