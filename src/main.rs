#[macro_use] extern crate serenity;

#[macro_use] extern crate lazy_static;

use std::env;
use std::collections::{HashSet, HashMap};
use std::str::FromStr;
use std::sync::Mutex;

use serenity::client::Client;
use serenity::framework::StandardFramework;
use serenity::prelude::*;
use serenity::model::prelude::*;
//use serenity::model::guild::{Role, Member, Guild};
//use serenity::model::channel::{GuildChannel, Message, Reaction};
//use serenity::model::id::MessageId;

struct Handler;

fn chan_by_name(guild:&Guild, name:&str)->Result<GuildChannel, ()>
{
    for (_,chan) in guild.channels().unwrap()
    {
        if chan.name == name
        {
            return Ok(chan);
        }
    }
    return Err(());
}

impl EventHandler for Handler
{
    fn guild_create(&self, _context: Context, guild: Guild, _is_new: bool)
    {
        println!("Guild: {}\n members: {:?}\n roles: {:?}",guild.name, guild.members, guild.roles);
        println!("Accessing properties");
        let ref mut bot=BOT.lock().unwrap()[0];

        println!("{}", bot.config.billboard_name);
        let board= chan_by_name(&guild, &bot.config.billboard_name).unwrap();
        let role= guild.role_by_name(&bot.config.hl_role_name).unwrap();
        bot.state=Some(State{
            events:HashMap::new(),
            billboard:board,
            hl_role:role.clone()
        });
    }

    fn reaction_add(&self, _context: Context, reaction: Reaction)
    {
        let bot_id=serenity::http::raw::get_current_user().unwrap().id;
        match BOT.lock().unwrap()[0].state
        {
            Some(ref mut st)=>
            {
                if let Some(event)=st.events.get_mut(&reaction.message_id)
                {
                    println!("reaction: {:?}", reaction);
                    if reaction.user_id != bot_id
                    {
                        event.subscribed.insert(reaction.user_id);
                    }
                }
            }
            _=>{}
        }
    }
}

struct Conf
{
    billboard_name: String,
    hl_role_name: String
}

#[derive(Debug)]
struct Event
{
    //id: u64,
    name: String,
    author: Member,
    details: String,
    subscribed: HashSet<UserId>
}

struct State
{
    //Registered events
    events: HashMap<MessageId, Event>,
    //Channel where announcements are posted
    billboard: GuildChannel,
    //Role included in mention for announcements
    hl_role: Role
}

struct Bot
{
    state: Option<State>,
    config: Conf
}

impl Bot
{
    fn new() -> Self
    {
        Bot
        {
            state:None,
            config: Conf
            {
                billboard_name: String::from_str("annonces").unwrap(),
                hl_role_name: String::from_str("Blitz").unwrap()
            }
        }
    }

}

lazy_static!
{
    static ref BOT: Mutex<Vec<Bot>> = Mutex::new(vec![]);
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut discord=Client::new(args[1].as_str(), Handler).expect("login failed");

    discord.with_framework(StandardFramework::new()
        .configure(|c|
            c.prefix("!")
            //.allowed_channels()
        )
        .cmd("say", say)
        .cmd("quit", quit)
        .cmd("print", print)
        .cmd("post", post)
    );

    let mut bot=Bot::new();
    BOT.lock().unwrap().push(bot);


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
            details:args.rest().to_owned(),
            subscribed:HashSet::new()
        };
        match BOT.lock().unwrap()[0].state
        {
            Some(ref mut st)=>
            {
                let message=st.billboard.say(&format!("<@&{}> {} posted the event {}:\n{}", &st.hl_role.id, event.author.display_name(), event.name, event.details)).unwrap();
                println!("New event: {}", event.details);
                if let Err(err)=message.react("✅")
                {
                    println!("Error reacting: {:?}", err);
                }

                st.events.insert(message.id, event);
            }
            _=>{}
        }
    }
);

command!(
    print(_context, message)
    {
        match BOT.lock().unwrap()[0].state
        {
            Some(ref st) =>
            {
                message.reply(&format!("{:?}", st.events));
            }
            None => {}
        }
    }
);
