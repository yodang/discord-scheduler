#[macro_use] extern crate serenity;

extern crate typemap;

use std::env;
use std::collections::{HashSet, HashMap};
use std::str::FromStr;

use serenity::client::Client;
use serenity::framework::StandardFramework;
use serenity::prelude::*;
use serenity::model::prelude::*;
//use serenity::model::guild::{Role, Member, Guild};
//use serenity::model::channel::{GuildChannel, Message, Reaction};
//use serenity::model::id::MessageId;

use typemap::Key;

struct Handler;

static CHECK_MARK: char='✅';


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
        let mut lock=_context.data.lock();

        let board= chan_by_name(&guild, &lock.get::<BotConf>().unwrap().billboard_name).unwrap();
        let role= guild.role_by_name(&lock.get::<BotConf>().unwrap().hl_role_name).unwrap();
        lock.insert::<BotState>(State{
            events:HashMap::new(),
            billboard:board,
            hl_role:role.clone()
        });
    }

    fn reaction_add(&self, _context: Context, reaction: Reaction)
    {
        let bot_id=serenity::http::raw::get_current_user().unwrap().id;
        match _context.data.lock().get_mut::<BotState>()
        {
            Some(ref mut st)=>
            {
                if let Some(event)=st.events.get_mut(&reaction.message_id)
                {
                    println!("reaction: {:?}", reaction);
                    if reaction.user_id != bot_id && reaction.emoji==ReactionType::Unicode(CHECK_MARK.to_string())
                    {
                        event.subscribed.insert(reaction.user_id);
                    }
                }
            }
            _=>{}
        }
    }

    fn reaction_remove(&self, _context: Context, reaction: Reaction)
    {
        match _context.data.lock().get_mut::<BotState>()
        {
            Some(ref mut st)=>
            {
                if let Some(event)=st.events.get_mut(&reaction.message_id)
                {
                    println!("reaction: {:?}", reaction);
                    if reaction.emoji==ReactionType::Unicode(CHECK_MARK.to_string())
                    {
                        event.subscribed.remove(&reaction.user_id);
                    }
                }
            }
            _=>{}
        }
    }
}

struct BotConf;

impl Key for BotConf
{
    type Value=Conf;
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

struct BotState;

impl Key for BotState
{
    type Value=State;
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

    discord.data.lock().insert::<BotConf>(Conf
    {
        billboard_name: String::from_str("annonces").unwrap(),
        hl_role_name: String::from_str("Blitz").unwrap()
    });


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
        match _context.data.lock().get_mut::<BotState>()
        {
            Some(ref mut st)=>
            {
                let message=st.billboard.say(&format!("<@&{}> {} posted the event {}:\n{}", &st.hl_role.id, event.author.display_name(), event.name, event.details)).unwrap();
                println!("New event: {}", event.details);
                if let Err(err)=message.react(CHECK_MARK)
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
        match _context.data.lock().get::<BotState>()
        {
            Some(ref st) =>
            {
                let _=message.reply(&format!("{:?}", st.events));
            }
            None => {}
        }
    }
);
