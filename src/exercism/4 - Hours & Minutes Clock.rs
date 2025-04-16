use std::fmt;

#[derive(PartialEq)]
pub struct Clock
{
    pub hours: i32,
    pub minutes: i32,
}

impl fmt::Debug for Clock
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "{:02}:{:02}", self.hours, self.minutes)
    }
}

impl Clock
{
    pub fn new(hours: i32, minutes: i32) -> Self
    {
        let total_minutes = hours * 60 + minutes;
        let normalized_minutes = total_minutes.rem_euclid(1440); // 1440 = 24*60

        let hours = normalized_minutes / 60;
        let minutes = normalized_minutes % 60;

        Clock { hours, minutes }
    }

    pub fn add_minutes(&self, minutes: i32) -> Self
    {
        Clock::new(self.hours, self.minutes + minutes)
    }

    pub fn to_string(&self) -> String
    {
        format!("{:02}:{:02}", self.hours, self.minutes)
    }
}

fn main()
{
    // Test cases
    let tests = [
        (10, 30, "10:30"),
        (34, 30, "10:30"), // 34 horas = 10 horas (34 % 24)
        (35, 30, "11:30"), // 35 horas = 11 horas (35 % 24)
        (5, 32, "05:32"),
        (5, 32 + 1500, "06:32"), // 1500 minutos = 25 horas
        (-54, -11513, "18:07"),  // Teste com valores negativos
        (-1, 15, "23:15"),       // 1 hora negativa
        (2, -30, "01:30"),       // 30 minutos negativos
        (-25, 0, "23:00"),       // 25 horas negativas
        (0, -160, "21:20"),      // 160 minutos negativos
    ];

    for (h, m, expected) in tests
    {
        let clock = Clock::new(h, m);
        println!("{:>4}h {:>5}m => {:?} (expected: {})", h, m, clock, expected);
        assert_eq!(clock.to_string(), expected);
    }
}
