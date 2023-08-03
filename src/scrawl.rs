use std::borrow::Cow;

fn abs_all(input: &mut Cow<[i32]>) {
    for i in 0..input.len() {
        let v = input[i];
        if v < 0 {
            // Clones into a vector if not already owned.
            input.to_mut()[i] = -v;
        }
    }
}

fn is_hello<T: AsRef<str>>(t: T) {
    assert_eq!(t.as_ref(), "hello");
}

#[derive(Copy)]
struct M { a: i32}

impl Clone for M {
    fn clone(&self) -> Self {
        M {a: self.a}
    }
}

fn int_mut(mut m: M) {
    m.a = 5;
    println!("int_mut m.a: {}", m.a);
}

fn int_mut_ref(m: &mut M) {
    m.a = 4;
    println!("int_mut_ref: {}", m.a);
}

fn vector_is_prime(num: u64, vec: Vec<u64>) -> bool {
    for i in vec {
        if num > i && num % i != 0 {
            return false;
        }
    }
    true
}

#[derive(Debug)]
struct Borrowed<'a>(&'a i32);


fn first_word<'a>(s: &'a str) -> &str {
    let bytes = s.as_bytes();
    for (i, &item) in bytes.iter().enumerate() {
        if item == b' ' {
            return &s[0..i];
        }
    }
    &s[..]
}
#[derive(Debug)]
struct A {
    a: i32,
}


#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::ops::Deref;
    use crate::scrawl::{A, abs_all, first_word, int_mut, int_mut_ref, is_hello, M, vector_is_prime};

    #[test]
    fn test_block() {
        let mut a: Option<A> = None;
        {
            let b = A{ a:3};
            a = Some(b);
        }
        println!("a: {:?}",  a);
    }

    #[test]
    fn test_abs_all() {
        let v1 = [1, 2, 3];
        let mut input = Cow::from(&v1[..]);
        abs_all(&mut input);
        println!("{:?}", input.deref());
    }

    #[test]
    fn test_as_ref() {
        is_hello(String::from("hello"));
        is_hello("hello");
    }

    #[test]
    fn test_mut() {
        let m = M { a: 3 };
        int_mut(m);
        println!("m.a: {}", m.a);

        let mut m1 = M { a: 4 };
        int_mut_ref(&mut m1);
        println!("m1.a: {}", m1.a);
    }

    #[test]
    fn test_vec_is_prime() {
        let mut count: u32 = 1;
        let mut num: u64 = 1;
        let mut primes: Vec<u64> = Vec::new();
        primes.push(2);

        while count < 10001 {
            num += 2;
            if vector_is_prime(num, primes.clone()) {
                primes.push(num);
            }
            count += 1;
        }
    }

    #[test]
    fn test_first_word() {
        let s = first_word("hello rustacean");
        println!("first_word: {}", s);
    }

}