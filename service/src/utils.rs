use rand::{prelude::SliceRandom, Rng};
use url::Url;

fn rand_vowel() -> char {
    let vowels = ['a', 'e', 'i', 'o', 'u'];
    let mut rng = rand::thread_rng();
    *vowels.choose(&mut rng).expect("failed to get a vowel") // this should never panic unless the slice is empty
}

fn rand_consonant() -> char {
    let consonants = [
        'b', 'c', 'd', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n',
        'p', 'q', 'r', 's', 't', 'v', 'w', 'x', 'y', 'z',
    ];
    let mut rng = rand::thread_rng();
    *consonants.choose(&mut rng).expect("failed to get a consonant") // this should never panic unless the slice is empty
}

pub (crate) fn generate_key(length: usize) -> String {
    let mut key = String::with_capacity(length);
    let random: bool = rand::thread_rng().gen();

    for i in 0..length {
        if i % 2 == (random as usize) {
            key.push(rand_consonant());
        } else {
            key.push(rand_vowel());
        }
    }

    key
}

pub (crate) fn is_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(parsed_url) => {
            // Check if the scheme is "http", "https", or "mailto" and if the host contains "."
            let scheme = parsed_url.scheme();
            let host = parsed_url.host_str().unwrap_or("");
            ["http", "https", "mailto"].contains(&scheme) && host.contains('.') && !host.contains("katb.in")
        }
        Err(_) => false,
    }
}
