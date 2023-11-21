use std::io::Write;

fn main() {
    loop {
        print!("json: ");
        std::io::stdout().flush().unwrap();
        let mut json = String::new();
        std::io::stdin().read_line(&mut json).unwrap();

        print!("path: ");
        std::io::stdout().flush().unwrap();
        let mut path = String::new();
        std::io::stdin().read_line(&mut path).unwrap();

        let json: serde_json::Value = serde_json::from_str(&json).unwrap();
        let path = sql_json_path::JsonPath::new(&path).unwrap();
        match path.query(&json) {
            Ok(values) => {
                for value in values {
                    println!("{}", value);
                }
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }
}
