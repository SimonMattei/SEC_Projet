use super::about;
use super::quit;

use super::db::enter_grade;
use super::db::show_grades;

use read_input::prelude::*;

pub fn student_action(teacher: &mut bool) {
    println!("*****\n1: See your grades\n2: Teachers' menu\n3: About\n0: Quit");
    let choice = input().inside(0..=3).msg("Enter Your choice: ").get();
    match choice {
        1 => show_grades("Enter your name. Do NOT lie!"),
        2 => become_teacher(teacher),
        3 => about(),
        0 => quit(),
        _ => panic!("impossible choice"),
    }
}

pub fn teacher_action() {
    println!("*****\n1: See grades of student\n2: Enter grades\n3 About\n0: Quit");
    let choice = input().inside(0..=3).msg("Enter Your choice: ").get();
    match choice {
        1 => show_grades("Enter the name of the user of which you want to see the grades:"),
        2 => enter_grade(),
        3 => about(),
        0 => quit(),
        _ => panic!("impossible choice"),
    }
}

pub fn become_teacher(teacher: &mut bool) {
    println!("Are you a prof? (yes/no) Do NOT lie!");
    let rep: String = input().get();
    if rep == "yes" {
        println!("Access allowed");
        *teacher = true;
    } else {
        println!("Access denied");
    }
}
