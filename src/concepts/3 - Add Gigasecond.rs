use time::Date;
use time::Duration;
use time::PrimitiveDateTime as DateTime;
use time::Time;

// Returns a DateTime one billion seconds after start.
pub fn after(start: DateTime) -> DateTime {
    start + Duration::seconds(1_000_000_000)
}

fn main() {
    let year: i32 = 2025;
    let month: time::Month = time::Month::March;
    let day: u8 = 26;
    let date = Date::from_calendar_date(year, month, day).unwrap();
    let time = Time::from_hms(0, 0, 0).unwrap();
    let dt = DateTime::new(date, time);
    println!("{:?}", dt);
    println!("One billion seconds later: {:?}", after(dt));
}
