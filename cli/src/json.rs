use fscore::Color;
use std::io::{self, Write};

pub enum Value<'a> {
    Str(&'a str),
    Num(String),
}

#[derive(Clone, Copy)]
pub struct Json {
    fmt: bool,
}

impl Json {
    pub fn new(fmt: bool) -> Self {
        Self { fmt }
    }

    fn paint(&self, out: &mut String, color: &str, text: &str) {
        if self.fmt {
            out.push_str(color);
            out.push_str(text);
            out.push_str(Color::reset());
        } else {
            out.push_str(text);
        }
    }

    fn indent(&self, out: &mut String, depth: usize) {
        if self.fmt {
            for _ in 0..depth {
                out.push_str("  ");
            }
        }
    }

    fn newline(&self, out: &mut String) {
        if self.fmt {
            out.push('\n');
        }
    }

    fn quote(text: &str) -> String {
        // Serializing a str cannot fail.
        serde_json::to_string(text).unwrap_or_else(|_| String::from("\"\""))
    }

    fn field(&self, out: &mut String, depth: usize, key: &str, value: &Value) {
        self.indent(out, depth);
        self.paint(out, Color::blue(), &Self::quote(key));
        out.push(':');

        if self.fmt {
            out.push(' ');
        }

        match value {
            Value::Str(text) => self.paint(out, Color::green(), &Self::quote(text)),
            Value::Num(text) => self.paint(out, Color::yellow(), text),
        }
    }

    fn object(&self, depth: usize, fields: &[(&str, Value)]) -> String {
        let mut out = String::from("{");

        for (index, (key, value)) in fields.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }

            self.newline(&mut out);
            self.field(&mut out, depth + 1, key, value);
        }

        self.newline(&mut out);
        self.indent(&mut out, depth);
        out.push('}');
        out
    }

    pub fn print(&self, fields: &[(&str, Value)]) {
        let mut out = self.object(0, fields);

        out.push('\n');
        emit(&out);
    }

    pub fn list(self) -> List {
        let mut out = String::from("{");

        self.newline(&mut out);
        self.indent(&mut out, 1);
        self.paint(&mut out, Color::blue(), "\"entries\"");
        out.push(':');

        if self.fmt {
            out.push(' ');
        }

        out.push('[');
        emit(&out);

        List {
            json: self,
            first: true,
        }
    }
}

pub struct List {
    json: Json,
    first: bool,
}

impl List {
    pub fn entry(&mut self, fields: &[(&str, Value)]) {
        let mut out = String::new();

        if !self.first {
            out.push(',');
        }

        self.json.newline(&mut out);
        self.json.indent(&mut out, 2);
        out.push_str(&self.json.object(2, fields));

        self.first = false;

        emit(&out);
    }

    pub fn finish(self, trailer: &[(&str, Value)]) {
        let json = self.json;
        let mut out = String::new();

        if !self.first {
            json.newline(&mut out);
            json.indent(&mut out, 1);
        }

        out.push(']');

        for (key, value) in trailer {
            out.push(',');
            json.newline(&mut out);
            json.field(&mut out, 1, key, value);
        }

        json.newline(&mut out);
        out.push_str("}\n");

        emit(&out);
    }
}

fn emit(text: &str) {
    let mut stdout = io::stdout().lock();

    let _ = stdout.write_all(text.as_bytes());
    let _ = stdout.flush();
}
