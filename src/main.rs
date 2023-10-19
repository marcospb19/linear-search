#![feature(array_windows)]

use std::{
    hint::black_box,
    sync::Arc,
    thread::{self, available_parallelism},
    time::Duration,
};

use fakeit::datetime;
use rand::{thread_rng, Rng};
use timeit::timeit_loops;

fn main() {
    run_benchmarks();
    run_tests();
}

fn run_benchmarks() {
    let text = black_box(include_str!("../shakespeare.txt"));
    // let nanos = black_box(datetime::nanosecond().to_string());
    let nanos = black_box(format!("Lorem"));
    // let nanos = black_box(format!("01/01/1970"));
    // let nanos = black_box(format!(
    //     "{nanos}{nanos}{nanos}{nanos}{nanos}{nanos}-{nanos}{nanos}{nanos}"
    // ));
    dbg!(&nanos);
    dbg!(nanos.len());
    // let nanos = &text[5458199 / 2..5458199 / 2 + 3];

    benchmark("std_search", || std_search(text, black_box(&nanos)));
    benchmark("naive_search", || naive_search(text, black_box(&nanos)));
    benchmark("sum_search", || sum_search(text, black_box(&nanos)));
    benchmark("bit_shift_search", || bit_shift_search(text, black_box(&nanos)));
    benchmark("sum_search_bad", || sum_search_bad(text, black_box(&nanos)));
    benchmark("xor_search", || xor_search(text, black_box(&nanos)));
    benchmark("needle", || needle_crate(text, black_box(&nanos)));
    benchmark("sum_3", || sum_search3(text, black_box(&nanos)));
    benchmark("boyer-moore-magiclen", || boyer_moore_magiclen(text, black_box(&nanos)));
    benchmark("threaded-sum-search", || threaded_sum_search(text, black_box(&nanos)));
    // benchmark("sum_search3", || sum_search3(text, black_box(&nanos)));
}

fn run_tests() {
    let text = black_box(include_str!("../shakespeare.txt"));

    // Test that all 3 algorithms give the same answer
    fn test(haystack: &str, needle: &str) -> bool {
        [
            std_search,
            naive_search,
            sum_search,
            bit_shift_search,
            sum_search_bad,
            xor_search,
            needle_crate,
            sum_search3,
            boyer_moore_magiclen,
            threaded_sum_search,
        ]
        .map(|f| f(haystack, needle))
        .array_windows::<2>()
        .all(|[a, b]| a == b)
    }

    // test matches
    assert!(test(text, &text[..1]));
    assert!(test(text, &text[..10]));
    assert!(test(text, &text[..100]));
    assert!(test(text, &text[..1000]));
    assert!(test(text, &text[1..5]));
    assert!(test(text, &text[5..10]));
    assert!(test(text, &text[100..1000]));
    assert!(test(text, &text[1000..10000]));
    assert!(test(text, &text[10000..text.len()]));

    // test non-matches
    assert!(test(text, "asjkdjkasndjkasd"));
    assert!(test(text, &datetime::nanosecond().to_string()));
    assert!(test(text, &datetime::nanosecond().to_string()));
    assert!(test(text, &datetime::nanosecond().to_string()));
    assert!(test(text, &datetime::nanosecond().to_string()));

    // test maybe-matches
    let mut rng = thread_rng();
    let mut mess_up = |text: &str| {
        let length_of_the_messing_up = rng.gen_range(1..4);
        let index_of_the_messing_up = rng.gen_range(0..text.len() - 5);
        let end_index_of_the_messing_up = index_of_the_messing_up + length_of_the_messing_up;

        let mut text = text.to_string();
        text.replace_range(index_of_the_messing_up..end_index_of_the_messing_up, "a");
        text
    };
    assert!(test(text, &mess_up(&text[..10])));
    assert!(test(text, &mess_up(&text[..100])));
    assert!(test(text, &mess_up(&text[..1000])));
    assert!(test(text, &mess_up(&text[100..1000])));
    assert!(test(text, &mess_up(&text[1000..10000])));
    assert!(test(text, &mess_up(&text[10000..text.len()])));
    assert!(test(text, &rng.gen_range::<u32, _>(0..1000).to_string()));
    assert!(test(text, &rng.gen_range::<u32, _>(0..1000).to_string()));
    assert!(test(text, &rng.gen_range::<u32, _>(0..1000).to_string()));
    assert!(test(text, &rng.gen_range::<u32, _>(0..1000).to_string()));
}

fn benchmark(function_name: &str, f: impl Fn() -> bool) {
    println!("{function_name} = {:.2?}", run_and_time_it(f));
}

fn std_search(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

// Time complexity: O(H * N)
// Space complexity: None
fn naive_search(haystack: &str, needle: &str) -> bool {
    let [haystack, needle] = [haystack, needle].map(str::as_bytes);

    haystack
        .windows(needle.len())
        .any(|haystack_window| haystack_window == needle)
}

// First implementation, worse code and worse performance, for some reason
fn sum_search_bad(haystack: &str, needle: &str) -> bool {
    // Needle length limit is u24::MAX (or u32::MAX / u8::MAX), because we're using u32 to hold the sum
    assert!(
        (needle.len() as u32).checked_mul(u8::MAX.into()).is_some(),
        "Sum of all chars of needle does not fit in u32"
    );

    // Treat corner cases
    if needle.is_empty() {
        return true;
    } else if needle.len() >= haystack.len() {
        return haystack == needle;
    }

    let needle_sum: u64 = needle.bytes().map(u64::from).sum();

    let mut haystack_iter = haystack.bytes();
    let mut haystack_sum: u64 = (&mut haystack_iter).take(needle.len()).map(u64::from).sum();

    let [haystack, needle] = [haystack, needle].map(str::as_bytes);

    for (left, _) in haystack_iter.enumerate() {
        let right = left + needle.len();

        if needle_sum == haystack_sum && &haystack[left..right] == needle {
            return true;
        }

        haystack_sum += haystack[right] as u64;
        haystack_sum -= haystack[left] as u64;
    }

    if needle_sum == haystack_sum && &haystack[haystack.len() - needle.len()..] == needle {
        return true;
    }

    false
}

fn sum_search(haystack: &str, needle: &str) -> bool {
    // Treat corner cases
    if needle.is_empty() {
        return true;
    } else if needle.len() >= haystack.len() {
        return haystack == needle;
    }

    let [haystack, needle] = [haystack, needle].map(str::as_bytes);

    let mut windows = haystack.windows(needle.len());

    // Unwrap Safety:
    //   We checked the size of the strings at the start of the function.
    let first_window = windows.next().unwrap();

    let sum_slice = |slice: &[u8]| -> u64 { slice.iter().copied().map(u64::from).sum() };
    let needle_sum = sum_slice(needle);
    let mut haystack_sum = sum_slice(first_window);

    if needle_sum == haystack_sum && first_window == needle {
        return true;
    }

    for (removed_element_index, window) in windows.enumerate() {
        // Unwrap Safety:
        //   We checked that needle length cannot be 0, therefore, a window has elements.
        haystack_sum += *window.last().unwrap() as u64;
        haystack_sum -= haystack[removed_element_index] as u64;

        // If the sum doesn't match, skip the check
        if needle_sum != haystack_sum {
            continue;
        }
        // Check equality
        if window == needle {
            return true;
        }
    }

    false
}

fn bit_shift_search(haystack: &str, needle: &str) -> bool {
    // Treat corner cases
    if needle.is_empty() {
        return true;
    } else if needle.len() >= haystack.len() {
        return haystack == needle;
    }

    if needle.len() > 64 {
        return sum_search(haystack, needle);
    }

    assert!(needle.len() <= 64);
    let bitmask = {
        let mut bitmask = 0;
        for _ in 0..needle.len() {
            bitmask <<= 1;
            bitmask |= 1;
        }
        bitmask
    };

    let [haystack, needle] = [haystack, needle].map(str::as_bytes);

    let mut windows = haystack.windows(needle.len());

    // Unwrap Safety:
    //   We checked the size of the strings at the start of the function.
    let first_window = windows.next().unwrap();

    let hash = |slice: &[u8]| -> u64 {
        let mut hash = 0;
        for byte in slice {
            let byte = *byte as u64;
            let bit = byte & 1;
            hash = hash << 1 | bit;
        }
        hash
    };
    let needle_hash = hash(needle);
    let mut haystack_hash = hash(first_window);

    if needle_hash == haystack_hash && first_window == needle {
        return true;
    }

    for window in windows {
        // Unwrap Safety:
        //   We checked that needle length cannot be 0, therefore, a window has elements.
        let new_element = *window.last().unwrap() as u64;

        let bit = new_element & 1;
        haystack_hash <<= 1;
        haystack_hash |= bit;
        haystack_hash &= bitmask;

        // haystack_hash -= haystack[removed_element_index] as u64;

        // If the sum doesn't match, skip the check
        if needle_hash != haystack_hash {
            continue;
        }
        // Check equality
        if window == needle {
            return true;
        }
    }

    false
}

fn sum_search3(haystack: &str, needle: &str) -> bool {
    let [haystack, needle] = [haystack, needle].map(str::as_bytes);

    let sum_slice = |slice: &[u8]| -> u64 { slice.iter().copied().map(u64::from).sum() };
    let needle_sum = sum_slice(needle);

    haystack.windows(needle.len()).any(|haystack_window| {
        let window_sum = sum_slice(haystack_window);
        window_sum == needle_sum && haystack_window == needle
    })
}

fn xor_search(haystack: &str, needle: &str) -> bool {
    // Treat corner cases
    if needle.is_empty() {
        return true;
    } else if needle.len() >= haystack.len() {
        return haystack == needle;
    }

    let [haystack, needle] = [haystack, needle].map(str::as_bytes);

    let mut windows = haystack.windows(needle.len());

    // Unwrap Safety:
    //   We checked the size of the strings at the start of the function.
    let first_window = windows.next().unwrap();

    let xor_slice = |slice: &[u8]| -> u64 { slice.iter().copied().map(u64::from).fold(0, |a, b| a ^ b) };
    let needle_sum = xor_slice(needle);
    let mut haystack_sum = xor_slice(first_window);

    if needle_sum == haystack_sum && first_window == needle {
        return true;
    }

    for (removed_element_index, window) in windows.enumerate() {
        // Unwrap Safety:
        //   We checked that needle length cannot be 0, therefore, a window has elements.
        haystack_sum ^= *window.last().unwrap() as u64;
        haystack_sum ^= haystack[removed_element_index] as u64;

        if needle_sum == haystack_sum && window == needle {
            return true;
        }
    }

    false
}

fn needle_crate(haystack: &str, needle: &str) -> bool {
    ::needle::BoyerMoore::new(needle.as_bytes())
        .find_first_in(haystack.as_bytes())
        .is_some()
}

fn boyer_moore_magiclen(haystack: &str, needle: &str) -> bool {
    boyer_moore_magiclen::BMByte::from(needle)
        .unwrap()
        .find_first_in(haystack)
        .is_some()
}

fn threaded_sum_search(haystack: &str, needle: &str) -> bool {
    const MIN_LEN_HEURISTIC: usize = 1024;

    if haystack.len() < MIN_LEN_HEURISTIC {
        sum_search(haystack, needle)
    } else {
        let haystack: Arc<str> = Arc::from(haystack);
        let needle: Arc<str> = Arc::from(needle);

        if needle.is_empty() {
            return true;
        }
        if needle.len() == haystack.len() {
            return haystack == needle;
        }
        if needle.len() > haystack.len() {
            return false;
        }

        let max_thread_count = available_parallelism().unwrap().get();

        let thread_count = (haystack.len() - needle.len()).min(max_thread_count);

        let mut threads = Vec::with_capacity(thread_count);
        for i in 0..thread_count {
            let haystack = haystack.clone();
            let needle = needle.clone();
            threads.push(thread::spawn(move || {
                let start = i * (haystack.len() - needle.len()) / thread_count;
                let end = ((i + 1) * haystack.len() + (thread_count - 1 - i) * needle.len()) / thread_count;
                sum_search(&haystack[start..end], &needle[..])
            }));
        }

        let mut contains = false;
        for thread in threads {
            contains |= thread.join().unwrap();
        }
        contains
    }
}

fn run_and_time_it(f: impl Fn() -> bool) -> Duration {
    // Warmup
    let _ = timeit_loops!(50 / 10, {
        black_box(f());
    });

    let duration = timeit_loops!(50, {
        black_box(f());
    });

    Duration::from_secs_f64(duration)
}
