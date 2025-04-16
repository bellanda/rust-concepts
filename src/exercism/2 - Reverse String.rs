pub fn reverse(s: &str) -> String
{
    s.chars().rev().collect()
}

fn main()
{
    println!("{}", reverse("stressed"));
    println!("{}", reverse("strops"));
    println!("{}", reverse("racecar"));
}
