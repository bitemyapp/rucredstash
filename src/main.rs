extern crate base64;
extern crate clap;
extern crate tokio_core;

use clap::{App, Arg, SubCommand};
use credstash::{CredStashClient, CredstashKey};
use std::ffi::OsString;
mod crypto;
use ring;
use std::clone::Clone;
use std::str;
use std::string::ToString;
use std::vec::Vec;
use tokio_core::reactor::Core;

// todo: change to CredstashApp
#[derive(Debug, PartialEq)]
struct RuCredStashApp {
    name: String,
    region_option: Option<String>,
    aws_profile: Option<String>,
    table_name: Option<String>,
    aws_arn: Option<String>,
    action: Action,
}

fn render_secret(secret: Vec<u8>) -> String {
    match str::from_utf8(&secret) {
        Ok(v) => v.to_string(),
        Err(_) => "".to_string(),
    }
}

fn render_comment(comment: Option<String>) -> String {
    match comment {
        None => "".to_string(),
        Some(val) => val,
    }
}

#[derive(Debug, PartialEq)]
enum Action {
    Delete(String),
    Get(String, Option<String>),
    GetAll,
    Keys,
    List,
    Put(String, String, Option<(String, String)>),
    Setup,
}

fn get_table_name(table_name: Option<String>) -> String {
    table_name.map_or("credential-store".to_string(), |name| name)
}

// todo: remove hardcoding here
fn handle_action(app: RuCredStashApp, client: CredStashClient) -> () {
    match app.action {
        Action::Put(credential_name, credential_value, encryption_context) => {
            let result = client.put_secret(
                get_table_name(app.table_name),
                credential_name,
                credential_value,
                None,
                encryption_context,
                None,
                None,
                ring::hmac::HMAC_SHA256,
            );
            let mut core = Core::new().unwrap();
            match core.run(result) {
                Ok(_) => println!("Item putten successfully"),
                Err(err) => eprintln!("Failure: {:?}", err),
            }
        }
        Action::List => {
            let result = client.list_secrets("credential-store".to_string());
            match result {
                Err(err) => println!("Failure: {:?}", err),
                Ok(val) => {
                    let newval = val.clone();
                    let max_name_len: Vec<usize> =
                        newval.into_iter().map(|item| item.name.len()).collect();
                    let max_len = max_name_len
                        .iter()
                        .fold(1, |acc, x| if acc < *x { *x } else { acc });
                    val.into_iter()
                        .map(|item| {
                            println!(
                                "{:width$} -- version {: <10} --comment {}",
                                item.name,
                                item.version,
                                render_comment(item.comment),
                                width = max_len
                            )
                        })
                        .collect::<Vec<()>>();
                }
            }
        }
        Action::Delete(credential) => {
            let result = client.delete_secret("credential-store".to_string(), credential);
            let mut core = Core::new().unwrap();
            match core.run(result) {
                Ok(_) => println!("Item deleted"),
                Err(err) => eprintln!("Failure: {:?}", err),
            }
        }
        Action::Setup => {
            let result = client.create_db_table("test-db".to_string(), "fixme".to_string());
            let mut core = Core::new().unwrap();
            match core.run(result) {
                Err(err) => eprintln!("Failure: {:?}", err),
                Ok(val) => {
                    println!("Creation initiated");
                }
            }
        }
        Action::Keys => {
            let result = client.list_secrets("credential-store".to_string());
            match result {
                Err(err) => eprintln!("Failure: {:?}", err),
                Ok(val) => {
                    let d: Vec<()> = val
                        .into_iter()
                        .map(|item| println!("{}", item.name))
                        .collect();
                }
            }
        }
        Action::Get(credential_name, _context) => {
            let mut core = Core::new().unwrap();
            let get_future = client.get_secret(
                "credential-store".to_string(),
                credential_name,
                ring::hmac::HMAC_SHA256,
            );
            match core.run(get_future) {
                Err(err) => eprintln!("Failure: {:?}", err),
                Ok(result) => eprintln!("{:?}", result),
            }
        }
        Action::GetAll => {
            let get_future = client.get_all_secrets("credential-store".to_string());
            let mut core = Core::new().unwrap();
            match core.run(get_future) {
                Err(err) => eprintln!("Failure: {:?}", err),
                Ok(val) => val
                    .into_iter()
                    .map(|item| {
                        println!(
                            "fetched: {} val: {}",
                            item.dynamo_name,
                            render_secret(item.dynamo_contents)
                        )
                    })
                    .collect(),
            }
        }
    }
}

impl RuCredStashApp {
    fn new() -> Self {
        Self::new_from(std::env::args_os().into_iter()).unwrap_or_else(|e| e.exit())
    }

    fn new_from<I, T>(args: I) -> Result<Self, clap::Error>
    where
        I: Iterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let app: App = App::new("rucredstash")
            .version("0.1")
            .about("A credential/secret storage system")
            .author("Sibi Prabakaran");

        let region_arg = Arg::with_name("region")
            .long("region")
            .short("r")
            .value_name("REGION")
            .help(
                "the AWS region in which to operate. If a region is \
                 not specified, credstash will use the value of the \
                 AWS_DEFAULT_REGION env variable, or if that is not \
                 set, the value in `~/.aws/config`. As a last resort, \
                 it will use us-east-1",
            );

        let table_arg = Arg::with_name("table")
            .long("table")
            .short("t")
            .value_name("TABLE")
            .help(
                "DynamoDB table to use for credential storage. If \
                 not specified, credstash will use the value of the \
                 CREDSTASH_DEFAULT_TABLE env variable, or if that is \
                 not set, the value `credential-store` will be used",
            );

        let profile_arg = Arg::with_name("profile")
            .long("profile")
            .short("p")
            .value_name("PROFILE")
            .help("Boto config profile to use when connecting to AWS");

        let arn_arg = Arg::with_name("arn")
            .long("arn")
            .short("n")
            .value_name("ARN")
            .help("AWS IAM ARN for AssumeRole");

        let del_command = SubCommand::with_name("delete")
            .about("Delete a credential from the store")
            .arg(Arg::with_name("credential").help("Delete a credential from the store"));

        let get_command = SubCommand::with_name("get")
            .about("Get a credential from the store")
            .arg(
                Arg::with_name("credential")
                    .help("the name of the credential to get")
                    .required(true)
            ).arg(
                Arg::with_name("context")
                    .help("encryption context key/value pairs associated with the credential in the form of key=value")

            )
            ;

        let get_all_command = SubCommand::with_name("getall")
            .about("Get all credentials from the store")
            .arg(Arg::with_name("secret").help("The secret to retrieve"));

        let keys_command = SubCommand::with_name("keys").about("List all keys in the store");

        let list_command =
            SubCommand::with_name("list").about("List credentials and their versions");

        let put_command = SubCommand::with_name("put")
            .about("Put a credential from the store")
            .arg(Arg::with_name("credential").help("the name of the credential to store"))
            .arg(Arg::with_name("value").help("the value of the credential to store"))
            .arg(Arg::with_name("context").help("encryption context key/value pairs associated with the credential in the form of key=value"))
            .arg(Arg::with_name("key").short("k").value_name("KEY").help("the KMS key-id of the master key to use. Defaults to alias/credstash"));

        let put_all_command = SubCommand::with_name("putall")
            .about("Put credentials from json into the store")
            .arg(Arg::with_name("secret").help("The secret to retrieve"));

        let setup_command = SubCommand::with_name("setup").about("setup the credential store");

        let app = app
            .arg(region_arg)
            .arg(table_arg)
            .arg(profile_arg)
            .arg(arn_arg)
            .subcommand(del_command)
            .subcommand(get_command)
            .subcommand(get_all_command)
            .subcommand(keys_command)
            .subcommand(list_command)
            .subcommand(put_command)
            .subcommand(put_all_command)
            .subcommand(setup_command);
        // extract the matches
        let matches: clap::ArgMatches = app.get_matches_from_safe(args)?;

        let region: Option<&str> = matches.value_of("region");
        let action_value: Action = match matches.subcommand() {
            ("get", Some(get_matches)) => {
                let credential: String = get_matches.value_of("credential").unwrap().to_string();
                let context = get_matches.value_of("context").map(|e| e.to_string());
                Action::Get(credential, context)
            }
            ("getall", _) => Action::GetAll,
            ("keys", _) => Action::Keys,
            ("list", _) => Action::List,
            ("setup", _) => Action::Setup,
            ("put", Some(put_matches)) => {
                let credential: String = put_matches.value_of("credential").unwrap().to_string();
                let value: String = put_matches.value_of("value").unwrap().to_string();
                let context: Option<String> =
                    put_matches.value_of("context").map(|e| e.to_string());
                let encryption_context: Option<(String, String)> =
                    context.map_or(None, |e| split_context_to_tuple(e));
                Action::Put(credential, value, encryption_context)
            }
            ("delete", Some(del_matches)) => {
                let credential: String = del_matches.value_of("credential").unwrap().to_string();
                Action::Delete(credential)
            }
            _ => unreachable!(),
        };

        Ok(RuCredStashApp {
            name: "Hello".to_string(),
            region_option: region.map(|r| r.to_string()),
            aws_profile: matches.value_of("profile").map(|r| r.to_string()),
            aws_arn: matches.value_of("arn").map(|r| r.to_string()),
            table_name: matches.value_of("table").map(|r| r.to_string()),
            action: action_value,
        })
        // panic!("undefined");
    }
}

fn split_context_to_tuple(encryption_context: String) -> Option<(String, String)> {
    let context: Vec<&str> = encryption_context.split('=').collect();
    match context.len() {
        0 => None,
        1 => panic!(
            "error: argument context: {} is not the form of key=value",
            encryption_context
        ),
        2 => Some((context[0].to_string(), context[1].to_string())),
        _ => panic!(
            "error: argument context: {} is not the form of key=value",
            encryption_context
        ),
    }
}

fn main() {
    let test = RuCredStashApp::new();
    println!("Hello, world {:?}", test);
    let client = CredStashClient::new();
    handle_action(test, client);

    // Get version
    // let version = client
    //     .get_highest_version("credential-store".to_string(), "hello".to_string())
    //     .unwrap();
    // println!("{}", version);

    // Put secret
    // let test = client.put_secret(
    //     "credential-store".to_string(),
    //     "testkey".to_string(),
    //     "0000000000000000001".to_string(),
    //     "testvalue".to_string(),
    //     None,
    //     None,
    //     ring::hmac::HMAC_SHA256,
    // );

    // println!("{:?}", test.unwrap());

    // Get Secret
    // let dynamo_row = client
    //     .get_secret(
    //         "credential-store".to_string(),
    //         "testkey".to_string(),
    //         ring::hmac::HMAC_SHA256,
    //     )
    //     .unwrap();

    // let secret = CredStashClient::decrypt_secret(dynamo_row);
    // let secret_utf8 = match str::from_utf8(&secret) {
    //     Ok(v) => v,
    //     Err(e) => panic!("invalid utf8 sequence: {}", e),
    // };

    // println!("{}", secret_utf8);

    // Delete secret
    // let dynamo_row = client
    //     .delete_secret("credential-store".to_string(), "hello".to_string())
    //     .unwrap();

    // list secrets
    // let dynamo_row = client
    //     .list_secrets("credential-store".to_string(), ring::hmac::HMAC_SHA256)
    //     .unwrap();
    // println!("{:?}", dynamo_row);
}
