use crossterm::{
    style::{Color, ResetColor, SetForegroundColor},
    ExecutableCommand,
};
use std::io::{stdin, stdout};

// Get user request
pub fn get_user_response(question: &str) -> String {
    let mut stdout: std::io::Stdout = stdout();

    // Print the question in a specific color
    stdout.execute(SetForegroundColor(Color::Blue)).unwrap();
    println!();
    println!("{}", question);

    // Reset Color
    stdout.execute(ResetColor).unwrap();

    // Read user input
    let mut user_response: String = String::new();
    /*
    stdin()
        .read_line(&mut user_response)
        .expect("Failed to read response");
    for line in stdin().lines() {
        user_response.push_str(&line.unwrap());
    }
    */
    for line in stdin().lines() {
        match line {
            Ok(line) => user_response.push_str(&line),
            Err(e) => panic!("{}", e),
        }
    }

    // Trim whitespace and return
    return user_response.trim().to_string();
}
