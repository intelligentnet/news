use std::collections::HashMap;

// Default delimiters
const START_DLIM: &str = "${";
const END_DLIM: &str = "}";

#[derive(Debug)]
pub struct Template<'a> {
    // Stores (key, (start, end))
    pub replaces: Vec<(&'a str, (usize, usize))>,
    pub template_str: &'a str,
    pub sdlim: &'a str,
    pub edlim: &'a str,
}

impl <'a> Template<'a> {
    pub fn new(template_str: &'a str) -> Self {
        Template::new_delimit(template_str, START_DLIM, END_DLIM)
    }

    pub fn new_delimit(template_str: &'a str, sdlim: &'a str, edlim: &'a str) -> Self {
        let template_str = template_str.trim();
        let mut template = Self { sdlim, edlim, replaces: Vec::new(), template_str };

        if template_str.is_empty() {
            return template;
        }

        let replaces = &mut template.replaces;

        // Current position in the format string
        let mut cursor = 0;

        while cursor <= template_str.len() {
            if let Some(start) = template_str[cursor..].find(sdlim) {
                let start = start + cursor;
                if let Some(end) = template_str[start..].find(edlim) {
                    let end = end + start;
                    replaces.push((
                        // The extracted key
                        &template_str[(start + sdlim.len())..end],
                        (start, (end + edlim.len())),
                    ));

                    // Move cursor to the end of this match
                    cursor = end + edlim.len();
                } else {
                    // Assume part of the text
                    break;
                }
            } else {
                replaces.push((
                    // The extracted key
                    &template_str[cursor..cursor], (cursor, cursor),
                ));
                break;
            }
        }
        template
    }

    pub fn render_strings(&self, vars: &HashMap<String, String>) -> String {
        let vars: HashMap<&str, &str> = vars.iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        
        self.recursive_render(&vars, 0)
    }

    pub fn render_env(&self) -> String {
        let vars: HashMap<&str, String> = HashMap::new();

        self.recursive_render(&vars, 0)
    }

    pub fn render<V: AsRef<str> + std::fmt::Debug + std::string::ToString>(&self, vars: &HashMap<&str, V>) -> String {
        self.recursive_render(vars, 0)
    }

    fn recursive_render<V: AsRef<str> + std::fmt::Debug + std::string::ToString>(&self, vars: &HashMap<&str, V>, level: u8) -> String {

        fn default<V: AsRef<str> + std::fmt::Debug + std::string::ToString>(key: &str, delimiter: &str, vars: &HashMap<&str, V>) -> String {
            let bits: Vec<_> = key.split(delimiter).collect();

            match vars.get(bits[0]) {
                Some(v) if !v.as_ref().is_empty() => 
                   v.to_string(),
                _ => {
                   match std::env::var(bits[0]) {
                       Ok(v) => v,
                       Err(_) => bits[1].to_string()
                   }
                }
            }
        }

        let replaces = &self.replaces;
        let template_str = &self.template_str;
        // Calculate the size of the text to be added (vs) and amount of space
        // the keys take up in the original text (ks)
        let (ks, vs) = replaces.iter().fold((0, 0), |(ka, va), (k, _)| {
            match vars.get(k) {
                Some(v) => {
                    (ka + k.len(), va + v.as_ref().len())
                },
                None =>  {
                    match std::env::var(k) {
                        Ok(v) => {
                            (ka + k.len(), va + v.len())
                        },
                        Err(_) => (ka + k.len(), va)
                    }
                }
            }
        });

        let final_len = (template_str.len() - (self.sdlim.len() * replaces.len())) + vs - ks;
        let mut output = String::with_capacity(final_len);
        let mut cursor: usize = 0;

        for (key, (start, end)) in replaces.iter() {
            output.push_str(&template_str[cursor..*start]);
            // Unwrapping is be safe at this point
            match vars.get(key) {
                Some(v) => {
                    output.push_str(v.as_ref())
                },
                None => {
                    // Implement default values if provided
                    if key.contains(":-") {
                        output.push_str(&default(key, ":-", vars));
                    } else if key.contains(":=") {
                        output.push_str(&default(key, ":=", vars));
                    } else {
                        match std::env::var(key) {
                            Ok(v) => output.push_str(v.as_ref()),
                            Err(_) => output.push_str("".as_ref())
                        }
                    }
                }
            }
            cursor = *end;
        }

        // If there's more text after the `${}`
        if cursor < template_str.len() {
            output.push_str(&template_str[cursor..]);
        }

        if level < 8 && output.contains(self.sdlim) {
            output = Template::new(&output).recursive_render(vars, level + 1);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once() {
        let test: &str = "Hello, ${name}, nice to meet you.";
        let mut args = HashMap::new();
        args.insert("name", "Charles");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "Hello, Charles, nice to meet you.");
    }

    #[test]
    fn beginning() {
        let test: &str = "${plural capitalized food} taste good.";
        let mut args = HashMap::new();
        args.insert("plural capitalized food", "Apples");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "Apples taste good.");
    }

    #[test]
    fn only() {
        let test: &str = "${why}";
        let mut args = HashMap::new();
        args.insert("why", "would you ever do this");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "would you ever do this");
    }

    #[test]
    fn end() {
        let test: &str = "I really love ${something}";
        let mut args = HashMap::new();
        args.insert("something", "programming");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "I really love programming");
    }

    #[test]
    fn empty() {
        let test: &str = "";
        let args:HashMap<&str, &str> = HashMap::new();

        let s = Template::new(test).render(&args);

        assert_eq!(s, "");
    }

    #[test]
    fn two() {
        let test: &str = "Hello, ${name}. You remind me of another ${name}.";
        let mut args = HashMap::new();
        args.insert("name", "Charles");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "Hello, Charles. You remind me of another Charles.");
    }

    #[test]
    fn twice() {
        let test: &str = "${name}, why are you writing code at ${time} again?";
        let mut args = HashMap::new();
        args.insert("name", "Charles");
        args.insert("time", "2 AM");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "Charles, why are you writing code at 2 AM again?");
    }

    #[test]
    fn default_empty() {
        let test: &str = "${name:-Henry}, why are you writing code at ${time} again?";
        let mut args = HashMap::new();
        //args.insert("name", "Charles");
        args.insert("time", "2 AM");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "Henry, why are you writing code at 2 AM again?");
    }

    #[test]
    fn default_some() {
        let test: &str = "${name:-Henry}, why are you writing code at ${time} again?";
        let mut args = HashMap::new();
        args.insert("name", "Charles");
        args.insert("time", "2 AM");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "Charles, why are you writing code at 2 AM again?");
    }

    #[test]
    fn recursive_empty() {
        let test: &str = "${name:-Henry}, why are you writing code at ${time} again?";
        let mut args = HashMap::new();
        args.insert("name", "${king:-Big Man}");
        args.insert("time", "2 AM");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "Big Man, why are you writing code at 2 AM again?");
    }

    #[test]
    fn recursive_some() {
        let test: &str = "${name:-Henry}, why are you writing code at ${time} again?";
        let mut args = HashMap::new();
        args.insert("king", "William");
        args.insert("name", "${king:-Big Man}");
        args.insert("time", "2 AM");

        let s = Template::new(test).render(&args);

        assert_eq!(s, "William, why are you writing code at 2 AM again?");
    }

    #[test]
    fn from_env() {
        let test: &str = "My name is ${NAME}";
        let s = Template::new(test).render_env();

        assert_eq!(s, "My name is ");

        std::env::set_var("NAME", "Henry");

        let s = Template::new(test).render_env();

        assert_eq!(s, "My name is Henry");
    }

    #[test]
    fn alone() {
        let mut args = HashMap::new();
        args.insert("dog", "woofers");

        let s = Template::new("${dog}").render(&args);

        assert_eq!(s, "woofers");
    }

    #[test]
    fn alt_delimeters() {
        let mut args = HashMap::new();
        args.insert("dog", "woofers");

        let s = Template::new_delimit("{dog}", "{", "}").render(&args);

        assert_eq!(s, "woofers");
    }
}
