use std::fs;

fn main()
{
    let file_path = "src/main.rs";
    let file_content = fs::read_to_string(file_path).expect("Failed to read file");
    println!("{}", file_content);
}
