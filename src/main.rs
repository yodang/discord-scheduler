#[macro_use] extern crate serenity;
use std::env;
use std::collections::HashSet;

use serenity::client::Client;
use serenity::framework::StandardFramework;
use serenity::prelude::*;
use serenity::model::guild::{Role, Member};
use serenity::model::channel::{GuildChannel, Message};

struct Handler;
impl EventHandler for Handler{}

struct Conf
{
    billboard_name: String,
    hl_role_name: String
}

struct Event
{
    //id: u64,
    name: String,
    author: Member,
    details: String,
    subscribed: HashSet<String>
}

struct State
{
    //Registered events
    events: Vec<Event>,
    //Channel where announcements are posted
    billboard: GuildChannel,
    //Role included in mention for announcements
    hl_role: Role
}

enum BotState
{
    Unitialized,
    Initialized(State)
}

struct Bot
{
    state: BotState,
    client: Client,
    config: Conf
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut discord=Client::new(args[1].as_str(), Handler).expect("login failed");

    discord.with_framework(StandardFramework::new()
        .configure(|c|
            c.allow_dm(false)
            .prefix("!")
            //.allowed_channels()
        )
        .cmd("say", say)
        .cmd("quit", quit)
    );

    if let Err(what)=discord.start()
    {
        println!("An error occured: {:?}", what);
    }

}

command!(
    say(_context, message)
    {
        let _=message.channel_id.say("Hello o/");
    }
);

command!(
    quit(context, _message)
    {
        context.quit();
    }
);

/*
 * Post event command
 * This command is given via DM and contains a description of the event.
 * The event is then saved and posted by the bot in the configured channel where guild members can
 * subscribe to it
 */
command!(
    //Only available via DM
    post(_context, message, args)
    {
        let event=Event {
            author:message.member().unwrap(),
            name:args.single_quoted::<String>().unwrap().to_owned(),
            details:args.full().to_owned(),
            subscribed:HashSet::new()
        };
    }
);
