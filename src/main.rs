use std::{
    collections::HashSet,
    fs,
    io::{stdin, stdout, Write},
    path::Path,
    sync::Arc,
};

use clap::Parser;
use colored::Colorize;
use futures::lock::Mutex;
use scout_interpreter::{
    builder::InterpreterBuilder,
    env::Env,
    object::{Object, ParseObj},
};
use scout_parser::ast::Identifier;

const TEMPLATE: &str = include_str!("template.sct");
const MAIN: &str = include_str!("main.sct");

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("\n        =================================================");
    println!("        =================               =================");
    println!(
        "        ================   {}   ================",
        "Trailblazer".green()
    );
    println!("        =================               =================");
    println!("        =================================================\n");
    println!("🌲🌳🌲 Starting our trail search at {} 🌲🌳🌲\n", args.url);

    if !Path::new("trailblazer").exists() {
        fs::create_dir("trailblazer")?;
    }

    let mut visited = HashSet::<String>::new();
    visited.insert(args.url.clone());

    let mut stack = vec![args.url.clone()];

    let env = Arc::new(Mutex::new(Env::default()));
    let inter = InterpreterBuilder::default()
        .with_env(env.clone())
        .build()
        .await?;

    let internal_start_url = Identifier::new("__startUrl".to_string());

    // Main loop. It pops from the URL stack, inserts the URLs from the stack
    // into the Scout environment and runs the MAIN script for each one. The output
    // of the MAIN script is a.) whether or not a `sign` function matched and b.)
    // the resulting URLs blazed. The URLs blazed are added to the stack if they
    // haven't already been visited.
    while let Some(next) = stack.pop() {
        env.lock()
            .await
            .set(&internal_start_url, Arc::new(Object::Str(next.clone())))
            .await;
        let res = inter.eval(MAIN).await.unwrap();
        visited.insert(next);

        // we know the output of the main.sct file since we fully control it.
        let parsed = <Object as ParseObj<Vec<Arc<Object>>>>::parse_obj(&res)
            .await
            .unwrap();

        if let Object::Boolean(false) = &*parsed[0] {
            // No sign functions matched the current URL
            let url = inter.current_url().await.unwrap();

            println!("❌ No signs were found! Unknown trail: \"{}\"", url);
            print!("\nInput a label for this webpage (or CTRL-c to exit): ");

            let _ = stdout().flush();
            let mut s = String::new();
            stdin().read_line(&mut s)?;

            s = s.trim().to_string();
            let filename = format!("trailblazer/{s}.sct");

            print!("Creating a new Trailblazer module at {filename}... ");

            let mut file = fs::File::create(filename)?;
            file.write_all(TEMPLATE.as_bytes())?;

            println!("✅");
            println!("Please update this file with any sign or trail logic!");
            break;
        } else {
            // A sign function did match the current URL and potentially
            // returned additional URLs to be visited.
            let next_url_objs = <Object as ParseObj<Vec<Arc<Object>>>>::parse_obj(&parsed[1])
                .await
                .unwrap();
            let mut next_urls: Vec<String> = Vec::new();
            for obj in next_url_objs {
                let s: String = <Object as ParseObj<String>>::parse_obj(&obj).await.unwrap();
                next_urls.push(s);
            }

            for url in next_urls {
                if !visited.contains(&url) {
                    stack.push(url);
                }
            }
        }
    }

    println!("You've ended up at your destination. Exiting!");

    inter.close().await;
    Ok(())
}

