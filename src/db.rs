use csv::{QuoteStyle, WriterBuilder};
use futures::executor::block_on;
use lazy_static::{__Deref, lazy_static};
use read_input::prelude::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

use log::{info, trace, warn};

pub mod big_array;
use big_array::BigArray;

pub mod hash;
use hash::padded_hash;
use hash::verify;

pub mod utils;
use utils::ask_for_email;
use utils::ask_for_name;
use utils::ask_for_pw;

pub mod access_control;

pub mod email;
use email::send_password_mail;
use rand::Rng;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(with = "BigArray")]
    pub pw_hash: [u8; 128],
    pub grades: Vec<f32>,
}

#[derive(Debug)]
pub struct UserDTO {
    pub id: String,
    pub email: String,
}

lazy_static! {
    static ref DATABASE: Mutex<Vec<User>> = {
        let data = read_database_from_file(DATABASE_FILE).unwrap_or(Vec::new());
        Mutex::new(data)
    };
}

const ADMIN_HASH: [u8; 128] = [
    36, 97, 114, 103, 111, 110, 50, 105, 100, 36, 118, 61, 49, 57, 36, 109, 61, 54, 53, 53, 51, 54,
    44, 116, 61, 50, 44, 112, 61, 49, 36, 56, 55, 81, 104, 72, 69, 100, 71, 51, 113, 120, 67, 118,
    82, 105, 56, 65, 115, 110, 85, 43, 65, 36, 78, 118, 68, 68, 53, 83, 89, 79, 109, 78, 118, 68,
    66, 74, 71, 88, 70, 109, 73, 87, 114, 98, 83, 99, 118, 69, 56, 115, 110, 75, 116, 106, 104,
    119, 72, 48, 56, 54, 111, 112, 99, 112, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const DATABASE_FILE: &str = "936DA01F9ABD4d9d80C702AF85C822A8.txt";
const NOT_ALLOWED_MSG: &str = "You are not allowed to do that!";

pub fn login() -> Option<UserDTO> {
    trace!("Login");

    let email = ask_for_email(true);

    let pw = input::<String>().msg("Please enter your password:\n").get();

    if email.eq("admin") {
        if verify(ADMIN_HASH, &pw) {
            return Some(UserDTO {
                email: String::from("admin"),
                id: String::from("admin"),
            });
        }
    } else {
        let data = DATABASE.lock().unwrap();
        for i in 0..(data.len()) {
            let user = &data[i];
            if user.email == email && verify(user.pw_hash, &pw) {
                return Some(UserDTO {
                    email: String::from(&user.email),
                    id: String::from(&user.id),
                });
            }
        }
    }

    return None;
}

pub fn create_account(user: &UserDTO, is_teacher_account: bool) {
    trace!("create_account");
    //check access control
    if is_teacher_account {
        if !block_on(access_control::auth(user, access_control::TEACHER_ACC)) {
            println!("{}", NOT_ALLOWED_MSG);
            return;
        }
    } else {
        if !block_on(access_control::auth(user, access_control::STUDENT_ACC)) {
            println!("{}", NOT_ALLOWED_MSG);
            return;
        }
    }

    //ask for info
    let email = &ask_for_email(false);
    let pw = ask_for_pw(false);
    let name = ask_for_name();
    let id = &Uuid::new_v4().to_string();

    //save in database
    let mut data = DATABASE.lock().unwrap();
    data.push(User {
        id: String::from(id),
        email: String::from(email),
        name: name,
        pw_hash: padded_hash(&pw),
        grades: Vec::new(),
    });

    //Write into access_control.csv a new teacher with access
    if is_teacher_account {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(access_control::POLICY)
            .unwrap();
        let mut wtr = WriterBuilder::new()
            .quote_style(QuoteStyle::Never)
            .from_writer(file);
        wtr.write_record(&[&format!("\ng, {}, teacher", id)])
            .unwrap();
        wtr.flush().unwrap();
        info!("{} Created a teacher account : {}", user.email, email)
    } else {
        info!("{} Created a student account : {}", user.email, email)
    }

    std::mem::drop(data);
    save_database_to_file();
}

pub fn reset_password(user: &UserDTO) {
    trace!("reset_password");

    if user.id.eq("admin") {
        println!("That's the one thing you cannot do! Change the password directly in the code");
        return;
    }

    println!("A token will been sent to the email");
    let mut rng = rand::thread_rng();
    let code = rng.gen_range(100000..999999);
    send_password_mail(&user.email, &code.to_string());
    info!("{} asked for a password change", user.email);

    //if we find the email, ask for it
    let code_entered = &input()
        .inside(100000..999999)
        .msg("Please enter the token sent (6 numbers):\n")
        .get();

    if code == *code_entered {
        let pw = ask_for_pw(true);
        let mut data = DATABASE.lock().unwrap();
        for i in 0..(data.len()) {
            let mut curr_user = &mut data[i];
            if curr_user.email.eq(&user.email) {
                curr_user.pw_hash = padded_hash(&pw);
                return;
            }
        }
        info!("Succesfull password reset for {}", user.email);
    } else {
        println!("Wrong code");
        warn!("Unsuccesfull password reset for {}", user.email)
    }

    save_database_to_file()
}

pub fn enter_grade(user: &UserDTO) {
    trace!("Enter_grade");

    if !block_on(access_control::auth(user, access_control::ENTER_GRADE)) {
        println!("{}", NOT_ALLOWED_MSG);
        return;
    }

    let email = ask_for_email(false);
    println!("What is the new grade of the student?");
    let grade: f32 = input().add_test(|x| *x >= 0.0 && *x <= 6.0).get();
    let mut data = DATABASE.lock().unwrap();
    for i in 0..(data.len()) {
        let user = &mut data[i];
        if user.email.eq(&email) {
            user.grades.push(grade);
            return;
        }
    }
    println!("No Student found with that email");
    warn!("No Student found with that email by {}", user.email);

    std::mem::drop(data);
    save_database_to_file();
}

pub fn show_grades(user: &UserDTO) {
    trace!("Show_grades");
    let mut is_teacher = true;

    if !block_on(access_control::auth(user, access_control::SHOW_GRADES)) {
        is_teacher = false;
    }

    let mut data = DATABASE.lock().unwrap();
    let mut res = "".to_owned();
    for i in 0..(data.len()) {
        let curr_user = &mut data[i];
        if !curr_user.grades.is_empty() && (is_teacher || user.id.eq(&curr_user.id)) {
            res.push_str(&format!(
                "{} : {:?} Mean : {}\n",
                curr_user.email,
                curr_user.grades,
                (curr_user.grades.iter().sum::<f32>()) / ((*curr_user.grades).len() as f32)
            ))
        }
    }

    println!("{}", res);
    info!("Successfully showed grades to {}", user.email);
}

pub fn read_database_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<User>, Box<dyn Error>> {
    trace!("Read_database");
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;
    Ok(data)
}

pub fn save_database_to_file() {
    trace!("Save_database");
    let file = File::create(DATABASE_FILE).unwrap();
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, DATABASE.lock().unwrap().deref()).unwrap();
}
