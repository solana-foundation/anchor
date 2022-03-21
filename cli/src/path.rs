#[macro_export]
macro_rules! home_path {
    ($my_struct:ident, $path:literal) => {
        #[derive(Clone, Debug)]
        pub struct $my_struct(String);

        impl Default for $my_struct {
            fn default() -> Self {
                match dirs::home_dir() {
                    None => {
                        println!("$HOME doesn't exist. This probably won't do what you want.");
                        $my_struct(".".to_string())
                    }
                    Some(mut path) => {
                        path.push($path);
                        $my_struct(path.as_path().display().to_string())
                    }
                }
            }
        }

        impl ToString for $my_struct {
            fn to_string(&self) -> String {
                self.0.clone()
            }
        }

        impl FromStr for $my_struct {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.to_string()))
            }
        }
    };
}
