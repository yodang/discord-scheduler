extern crate discord;
use std::env;
use discord::Discord;
use discord::model::Event;

fn main() {
    let discord=Discord::from_bot_token(env::args().nth(1)).expect("login failed");

    let (mut connection, _)=discord.connect().expect("connection failed");
    println!("Connected!");

    loop {
        match connection.recv_event() {
            Ok(Event::MessageCreate(message)) => 
            {
                println("{} says: {}", message.author.name, message.content);
                if(message.content == "!say")
                    discord.send_message(message.channel_id, "Hello o/", "", false);
                else if(message.content == "!quit")
                {
                    println!("Quitting");
                    break;
                }

            }
            Ok(_) => {}
            Err(discord::Error::closed(code, body)) =>
            {
                println!("Gateway sent error with code {:?}: {}", code, body);
                break;
            }
            Err(err) =>
            {
                println!("Received error: {:?}", err);
            }
        }
    }
}