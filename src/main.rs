#[macro_use] extern crate serenity;
use std::env;
use std::collections::HashSet;

use serenity::client::Client;
use serenity::framework::StandardFramework;
use serenity::prelude::*;
use serenity::model::guild::{Role, Member};
use serenity::model::channel::{GuildChannel, Message};

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
