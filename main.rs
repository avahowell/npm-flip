extern crate rand;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufReader,BufRead};
use std::env;
use std::fs::File;


// is_cyclical checks if a bitflipped package imports its original
fn is_cyclical(flipped_name: &String, package_name: &String) -> bool {
    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    let registry_url = format!("http://registry.npmjs.org/{}", flipped_name).parse().unwrap();
    let work = client.get(registry_url).and_then(|res| {
        return res.body().concat2().map(|body| {
            let data :Value = serde_json::from_slice(&body).unwrap();
            if let Some(latest_tag) = data["dist-tags"]["latest"].as_str() {
                if data["versions"][latest_tag]["dependencies"][package_name.as_str()].is_string() ||
                    data["versions"][latest_tag]["devDependencies"][package_name.as_str()].is_string() {
                        return true
                    }
            }
            return false
        })
    });
    return core.run(work).unwrap();
}

fn flip_str(input: &String, character_index :usize, shift: u8) -> Option<String> {
    let mut string_bytes = vec![0;input.len()];
    string_bytes.copy_from_slice(&input.as_bytes());

    let flip_mask: u8 = 1 << shift;
    string_bytes[character_index] ^= flip_mask;

    return String::from_utf8(string_bytes).ok();
}

fn flip_exhaustive(original: &String) -> Vec<String> {
    let mut flips = Vec::new();
    for character in 0..original.len() {
        for shift in 0..8 {
            if let Some(flipped) = flip_str(original, character, shift) {
                flips.push(flipped);
            }
        }
    }
    flips 
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("requires two arguments: flip [target-packages] [all-packages]");
        return
    }

    let target_packages_file = File::open(&args[1]).expect("could not open target packages file");
    let all_packages_file = File::open(&args[2]).expect("could not open all packages file");

    let mut all_packages = HashMap::new();
    let mut target_packages = Vec::new();
    for line in BufReader::new(target_packages_file).lines() {
        target_packages.push(line.unwrap())
    }
    for line in BufReader::new(all_packages_file).lines() {
        all_packages.insert(line.unwrap(), true);
    }

    println!("searching for bitflips...");

    let mut found = Vec::new();
    for package_name in target_packages {
        for flipped_name in &flip_exhaustive(&package_name) {
            if all_packages.contains_key(flipped_name) {
                println!("found bitflip: {} -> {}", &package_name, &flipped_name);
                found.push((flipped_name.clone(),package_name.clone()));
            }
        }
    }

    println!("checking for suspicous bitflipped dependencies");
    for package in found {
        let flipped_name = package.0;
        let package_name = package.1;
        if is_cyclical(&flipped_name, &package_name) {
            println!("CYCLICAL BITFLIP: {}", &flipped_name);
        }
    }
}
