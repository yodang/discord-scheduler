#[macro_use] extern crate serenity;

extern crate typemap;

extern crate serde;
extern crate serde_json;

use std::env;
use std::collections::{HashSet, HashMap};
use std::str::FromStr;
use std::ops::Drop;

use serenity::client::Client;
use serenity::framework::StandardFramework;
use serenity::prelude::*;
use serenity::model::prelude::*;
//use serenity::model::guild::{Role, Member, Guild};
//use serenity::model::channel::{GuildChannel, Message, Reaction};
//use serenity::model::id::MessageId;

use typemap::Key;

use serde::{Serialize, Deserialize};

struct Handler;

//Missing functionnality:
//-Save/Restore state (list off monitored message ids and their author)

static CHECK_MARK: &str="✅";
static REMOVE_MARK: &str="❌";


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
            votes:HashMap::new(),
            billboard:board,
            hl_role:role.clone()
        });
    }

    fn reaction_add(&self, _context: Context, reaction: Reaction)
    {
        event_react_add(&_context, &reaction);
        vote_react_add(&_context, &reaction);
    }

    fn reaction_remove(&self, _context: Context, reaction: Reaction)
    {
        event_react_remove(&_context, &reaction);
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

#[derive(Debug, Serialize, Deserialize)]
struct Event
{
    //id: u64,
    name: String,
    author: Member,
    guild: GuildId,
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
    //Current votes
    votes: HashMap<MessageId, Vote>,
    //Channel where announcements are posted
    billboard: GuildChannel,
    //Role included in mention for announcements
    hl_role: Role
}

impl Drop for State
{
    fn drop(&mut self)
    {
        println!("State is dropped, here it is:");
        println!("{}", serde_json::to_string_pretty(&self.events).unwrap());
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut discord=Client::new(args[1].as_str(), Handler).expect("login failed");

    discord.with_framework(StandardFramework::new()
        .configure(|c|
            c.prefix("!")
            //.allowed_channels()
        )
        .cmd("quit", quit)
        .cmd("print", print)
        .cmd("post", post)
        .cmd("my_events", my_events)
        .cmd("callvote", callvote)
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
    println!("Exiting, here is the state:");
    println!("{}", serde_json::to_string_pretty(&discord.data.lock().get::<BotState>().unwrap().events).unwrap());
}

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
    post(_context, message, args)
    {
        let event=Event {
            author:message.member().unwrap(),
            name:args.single_quoted::<String>().unwrap().to_owned(),
            guild: message.guild_id.unwrap(),
            details:args.rest().to_owned(),
            subscribed:HashSet::new()
        };
        match _context.data.lock().get_mut::<BotState>()
        {
            Some(ref mut st)=>
            {
                let msg=st.billboard.say(&format!("<@&{}> {} posted the event {}:\n{}", &st.hl_role.id, event.author.display_name(), event.name, event.details)).unwrap();
                println!("New event: {}", event.details);
                if let Err(err)=msg.react(CHECK_MARK)
                {
                    println!("Error reacting: {:?}", err);
                }
                if let Err(err)=msg.react(REMOVE_MARK)
                {
                    println!("Error reacting: {:?}", err);
                }
                st.events.insert(msg.id, event);
                let _=message.delete();
            }
            _=>{}
        }
    }
);

fn event_react_add(_context: &Context, reaction: &Reaction)
{
    let bot_id=serenity::http::raw::get_current_user().unwrap().id;
    println!("reaction: {:?}", reaction);
    match reaction.emoji
    {
        ReactionType::Unicode(ref s) if s==CHECK_MARK =>
        {
            if reaction.user_id != bot_id
            {
                match _context.data.lock().get_mut::<BotState>()
                {
                    Some(ref mut st)=>
                    {
                        if let Some(event)=st.events.get_mut(&reaction.message_id)
                        {
                            event.subscribed.insert(reaction.user_id);
                        }
                    }
                    _=>{}
                }
            }
        }
        ReactionType::Unicode(ref s) if s==REMOVE_MARK =>
        {
            match _context.data.lock().get_mut::<BotState>()
            {
                Some(ref mut st)=>
                {
                    let mut remove=false;
                    if let Some(event)=st.events.get(&reaction.message_id)
                    {
                        if reaction.user_id == event.author.user_id()
                        {
                            let _=reaction.message().unwrap().delete();
                            remove=true;
                        }
                    }
                    if remove
                    {
                        st.events.remove(&reaction.message_id);
                    }
                }
                _=>{}
            }
        }
        _ => {}
    }
}

fn event_react_remove(_context: &Context, reaction: &Reaction)
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

command!(
    print(_context, message)
    {
        //Only available via DM
        if !message.is_private()
        {
            return Ok(());
        }

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

fn print_nicks<'a, I>(users: I, guild: &GuildId) -> String
where
    I:Iterator<Item = &'a UserId>
{
    users.fold("".to_owned(), |acc, u|
    {
        println!("User: {:?}", u);
        format!("{}{}, ", acc, u.to_user().unwrap().
        nick_in(guild).unwrap_or_else(||{u.to_user().unwrap().name}))
    })
}

command!(
    my_events(context, message)
    {
        //Only available via DM
        if !message.is_private()
        {
            return Ok(());
        }

        match context.data.lock().get::<BotState>()
        {
            Some(ref st) =>
            {
                let content=st.events.iter().fold("".to_owned(), |acc, (_, event)|
                {
                    if event.author.user_id() == message.author.id
                    {
                        format!("{}\n{}: {}", acc, event.name, print_nicks(event.subscribed.iter().map(|a|{a}), &event.guild))
                    }
                    else
                    {
                        acc
                    }
                });
                let _=message.reply(&content);
            }
            None => {}
        }
    }
);

//TODO: find a way to end/delete a vote
struct Vote{
    desc: String,
    results: Vec<(String, u32)>,
    msg: MessageId
    //guild: GuildId,
    //author: Member
}

impl Vote{
    fn text(self: &Vote) -> String
    {
        let mut results_str="".to_owned();
        let mut pos=1;
        for (choice, count) in self.results.iter()
        {
            results_str.push_str(&format!("{}-{}: {}\n", pos, choice, count));
            pos+=1;
        }
        format!("A vote started {}:\n{}", self.desc, results_str)
    }
}

fn vote_react_add(_context: &Context, reaction: &Reaction)
{
    let bot_id=serenity::http::raw::get_current_user().unwrap().id;
    if reaction.user_id == bot_id
    {
        return;
    }
    println!("vote reaction: {:?}", reaction);
    match _context.data.lock().get_mut::<BotState>()
    {
        Some(ref mut st)=>
        {
            if let Some(vote)=st.votes.get_mut(&reaction.message_id)
            {
                match reaction.emoji
                {
                    ReactionType::Unicode(ref s) =>
                    {
                        if s.ends_with("\u{20e3}")
                        {
                            if let Some(n)=s.chars().next().unwrap().to_digit(10)
                            {
                                let i=n as usize;
                                println!("Got reaction {}", i);
                                if i>0 && i<=vote.results.len()
                                {
                                    let (s, count)=&vote.results[i-1];
                                    println!("Increasing entry ({},{})", s, count);
                                    vote.results[i-1]=(s.to_string(), count+1);
                                    let content=format!("<@&{}> {}", &st.hl_role.id, vote.text());
                                    st.billboard.edit_message(vote.msg, |m| m.content(&content)).unwrap();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

command!(
    callvote(context, message, args)
    {
        //Only available via DM
        if !message.is_private()
        {
            return Ok(());
        }

        match context.data.lock().get_mut::<BotState>()
        {
            Some(ref mut st)=>
            {
                let msg=st.billboard.say(&format!("<@&{}>", &st.hl_role.id)).unwrap();
                let vote=Vote {
                    //author:message.member().unwrap(),
                    desc:args.single_quoted::<String>().unwrap().to_owned(),
                    //guild: message.guild_id.unwrap(),
                    results:args.multiple_quoted::<String>().unwrap().into_iter().map(|x| (x,0)).collect(),
                    msg:msg.id
                };
                println!("New event: {}", vote.desc);
                st.billboard.edit_message(vote.msg, |m| m.content(&format!("<@&{}> {}", &st.hl_role.id, vote.text()))).unwrap();
                for i in 1..(vote.results.len()+1) //Ranges upper bound are excluded
                {
                    if let Err(err)=msg.react(format!("{}\u{20e3}", i))
                    {
                        println!("Error reacting: {:?}", err);
                    }
                }
                st.votes.insert(msg.id, vote);
            }
            _=>{}
        }
    }
);
