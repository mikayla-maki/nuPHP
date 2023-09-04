use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Response {
    pub headers: Option<Vec<(String, String)>>,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct NuRecord<'a, 'b>(&'a [(&'b str, &'b str)]);

impl<'a, 'b> NuRecord<'a, 'b> {
    pub fn of(map: &'a [(&'b str, &'b str)]) -> Self {
        Self(map)
    }
}

impl<'a, 'b> Display for NuRecord<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (key, val) in self.0 {
            writeln!(f, "{}: {}", key, val)?;
        }
        write!(f, "}}")
    }
}
