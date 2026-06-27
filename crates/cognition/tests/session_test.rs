use loop_engineering_cognition::client::Role;
use loop_engineering_cognition::session::Session;

#[test]
fn test_session_create_add() {
    let mut session = Session::new("loop-1", "unit-1");
    session.add_message(Role::User, "Hello".to_string());
    
    let messages = session.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello");
}

#[test]
fn test_session_stats() {
    let mut session = Session::new("loop-1", "unit-1");
    session.add_message(Role::User, "msg1".to_string());
    session.add_message(Role::Assistant, "msg2".to_string());
    
    let stats = session.stats();
    assert_eq!(stats.message_count, 2);
    assert_eq!(stats.loop_id, "loop-1");
    assert_eq!(stats.unit_id, "unit-1");
}

#[test]
fn test_session_clear() {
    let mut session = Session::new("loop-1", "unit-1");
    session.add_message(Role::User, "Hello".to_string());
    session.clear();
    
    let messages = session.get_messages();
    assert_eq!(messages.len(), 0);
}

#[test]
fn test_session_with_messages() {
    let mut session = Session::new("loop-1", "unit-1");
    session.add_message(Role::System, "You are helpful".to_string());
    session.add_message(Role::User, "Hi".to_string());
    session.add_message(Role::Assistant, "Hello!".to_string());
    
    let messages = session.get_messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].role, Role::System);
    assert_eq!(messages[2].role, Role::Assistant);
}
