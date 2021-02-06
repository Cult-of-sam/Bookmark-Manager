use serde_derive::{
    Serialize,
    Deserialize,
};

#[macro_use]
extern crate clap;

use clap::{
    Arg,
    SubCommand,
    ArgMatches};

use std::{
    cmp::Ordering,
    error::Error,
    fmt::{
        self,
        Display,
        Formatter,
    },
    fs::{
        File,
        OpenOptions,
    },
    io::{
        Seek,
        SeekFrom,
        stdout,
        Write,
    },
    path::Path,
};

/*
1. Add/update bookmark(if it already exists)

2. Removing a record by name.

3. Outputting a book records bookmark to a text file for the main program
 */

fn main() -> Result<(), String> {
    let matches = app_from_crate!()
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .value_name("FILE")
            .help("The file to read/write to")
            .default_value("bookmarks"))
        .arg(Arg::with_name("output")
            .long("output-file")
            .value_name("FILE")
            .help("The file to write the output to")
            .default_value("-"))
        .subcommand(SubCommand::with_name("add")
            .about("Add a new bookmark")
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("NAME")
                .help("The name of the bookmark to add/update")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("offset")
                .short("o")
                .long("offset")
                .value_name("OFFSET")
                .help("The time offset to save")
                .takes_value(true)
                .required(true)))
        .subcommand(SubCommand::with_name("remove")
            .about("Remove an existing bookmark")
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("NAME")
                .help("The name of the bookmark to remove")
                .takes_value(true)
                .required(true)))
        .subcommand(SubCommand::with_name("query")
            .about("Get the value of an existing bookmark")
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("NAME")
                .help("The name of the bookmark to search for")
                .takes_value(true)
                .required(true)))
        .get_matches();

    let return_value = control(&matches);

    match return_value {
        Ok(Some(val)) => {
            let output = value_t!(matches, "output", String).unwrap_or("-".into());

            let mut output_writer: Box<dyn Write> = if output == "-" {
                Box::new(stdout())
            } else {
                Box::new(File::create(output).unwrap())
            };

            writeln!(&mut output_writer, "{:?}", val).unwrap();
            Ok(())
        },
        Ok(None) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

fn control(matches: &ArgMatches) -> Result<Option<Bookmark>, Box<dyn Error>> {
    match matches.subcommand_name() {
        Some("add") => {
            let filename = value_t!(matches, "file", String)?;

            let matches = matches.subcommand_matches("add").ok_or(ManagerError::new("Failed to find add subcommand"))?;
            let bookmark_name = value_t!(matches, "name", String)?;
            let offset = value_t!(matches, "offset", f64)?;

            add_bookmark(filename, bookmark_name, offset)
        },
        Some("remove") => {
            let filename = value_t!(matches, "file", String)?;

            let matches = matches.subcommand_matches("remove").ok_or(ManagerError::new("Failed to find remove subcommand"))?;
            let bookmark_name = value_t!(matches, "name", String)?;

            remove_bookmark(filename, bookmark_name)
        },
        Some("query") => {
            let filename = value_t!(matches, "file", String)?;

            let matches = matches.subcommand_matches("query").ok_or(ManagerError::new("Failed to find query subcommand"))?;
            let bookmark_name = value_t!(matches, "name", String)?;

            query_bookmark(filename, bookmark_name)
        },
        _ => Err(Box::new(ManagerError::new("Unable to match subcommand")))
    }
}

fn add_bookmark<P: AsRef<Path>>(file: P, name: String, offset: f64) -> Result<Option<Bookmark>, Box<dyn Error>> {
    let mut file = OpenOptions::new().read(true).write(true).create(true).open(file)?;
    let mut bookmarks: Vec<Bookmark> = serde_yaml::from_reader(&file).unwrap_or(Vec::new());

    if bookmarks.iter().find(|x| x.name == name).is_none() {
        bookmarks.push(Bookmark::new(name, offset));
    } else {
        bookmarks.iter_mut().find(|x| x.name == name).unwrap().offset = offset;
    }

    bookmarks.sort_unstable_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap_or(Ordering::Equal));
    bookmarks.dedup_by(|x, y| x.name == y.name);

    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;

    serde_yaml::to_writer(&file, &bookmarks)?;
    Ok(None)
}

fn remove_bookmark<P: AsRef<Path>>(file: P, name: String) -> Result<Option<Bookmark>, Box<dyn Error>> {
    let mut file = OpenOptions::new().read(true).write(true).open(file)?;
    let mut bookmarks: Vec<Bookmark> = serde_yaml::from_reader(&file)?;

    if let Some(index) = bookmarks.iter().position(|x| x.name == name) {
        let bookmark: Bookmark = bookmarks.remove(index);

        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;

        serde_yaml::to_writer(&file, &bookmarks)?;
        return Ok(Some(bookmark));
    }
    Ok(None)
}

fn query_bookmark<P: AsRef<Path>>(file: P, name: String) -> Result<Option<Bookmark>, Box<dyn Error>> {
    let file = OpenOptions::new().read(true).write(true).open(file)?;
    let mut bookmarks: Vec<Bookmark> = serde_yaml::from_reader(&file)?;

    let bookmark: Option<Bookmark> = bookmarks.drain(..).filter(|val| val.name == name).next();

    Ok(bookmark)
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Debug)]
pub struct Bookmark {
    name: String,
    offset: f64,
}

impl Bookmark {
    pub fn new(name: String, offset: f64) -> Bookmark {
        Bookmark {
            name,
            offset,
        }
    }
}

#[derive(Debug)]
struct ManagerError {
    message: String,
}

impl ManagerError {
    pub fn new<T: Into<String>>(message: T) -> ManagerError {
        ManagerError {
            message: message.into(),
        }
    }
}

impl Error for ManagerError {}

impl Display for ManagerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}