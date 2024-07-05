/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==============================================================================
// Imports
//==============================================================================

use nanvix::pm::{
    self,
    ffi,
    Tid,
};

use core::slice;

//==============================================================================
// Constants
//==============================================================================

// Prandom semaphore Key.
const PRANDOM_KEY: u32 = 41274;

// Semaphore operations.
const SEMAPHORE_UP: u32 = 0;
const SEMAPHORE_DOWN: u32 = 1;

// Semaphore Contol.
const SET_COUNT: u32 = 1;

// Number of semaphores used in readers and writers .
const SEMAPHORE_TOTAL: usize = 5;

// Number of readers and writers.
const READERS_TOTAL: usize = 10;
const WRITERS_TOTAL: usize = 3;

// Number of operation that will be executed
// by each thread.
const OP_READER: u32 = 20;
const OP_WRITER: u32 = 5;

// Buffer Lenght
const BUFFER_LENGHT: usize = 30;

// Number of primes returned by find_prime().
const PRIMES_LENGHT: usize = 5;

// Limit value for search prime numbers.
const PRIME_LIMIT: u32 = 2500;

//==============================================================================
// Private Variables
//==============================================================================

// Primes Buffer.
static mut BUFFER: [u32; BUFFER_LENGHT] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
];

//==============================================================================
// Private Standalone Functions
//==============================================================================

///
/// ** Description **
///
/// Generate pseudo-random numbers (thread safe).
///
/// This function use Linear congruential for
/// generate pseudorandom numbers smaller than m.
///
fn prandom(rand: &mut u32) {
    // Get the prandom's semaphore id, already
    // initialized by readerswriters().
    let sem: i32 = pm::semget(PRANDOM_KEY);

    // Random number to be the seed in each interation.
    static mut NEXT: u32 = 42;

    // Random number to be the multiplier.
    let a: u32 = 4847;

    // Random number to be the increment.
    let b: u32 = 2351;

    // Random number to be the modulus.
    let m: u32 = 99999;

    pm::semop(sem as u32, SEMAPHORE_DOWN);

    unsafe {
        // Calculates next pseudo-random number.
        NEXT = ((a * NEXT) + b) % m;

        // Set new number generated in referenced variable, .
        *rand = NEXT;
    }

    pm::semop(sem as u32, SEMAPHORE_UP);
}

///
/// ** Description **
///
/// Calc the aproximate square root of n, where n > 0.
///
/// This function uses Heron's Method. The method
/// is, we choose a estimate x about the root of n
/// where the range between x and n contains root of n.
/// So, if the x is a underestimate to the square root,
/// and n/x is a overestimate, or vice versa, so the average
/// of these two numbers is a best aproximation of the
/// square root of n. Now we replace value of x with the new
/// best estimate and interate again.
///
/// This implemenation stop or, when the aproximation is equal the
/// new aproximation found, both truncated, since we need just
/// a integer number, or stop when the limit of interations is
/// reached.
///
fn sqrt_heron(n: u32) -> u32 {
    // Guess a number for square root of n.
    let mut x = 1;

    // Arbitrary limit of interations.
    let mut limit = 1000;

    // Find a better guess.
    while limit > 0 {
        // Calculates Heron's formula and truncate
        // float result using cast.
        let better_guess = ((x + (n / x)) as f32 * 0.5) as u32;

        // Stop interations if the value of new guess is 0
        // or the last guess is equal the new guess (no change).
        if better_guess != 0 && better_guess != x {
            x = better_guess;
        } else {
            break;
        }

        limit -= 1;
    }
    return x;
}

///
/// *Description*
///
/// Return the last primes up to n. The number of primes returned
/// is defined by the constant PRIMES_LENGHT.
///
/// This function use Sieve of Eratosthenes for calculates
/// all the primes up to n and return the last found.
///
fn find_primes(n: u32) -> [u32; PRIMES_LENGHT] {
    // Max static array lenght.
    if n > PRIME_LIMIT as u32 {
        return [0; PRIMES_LENGHT];
    }

    // Create a static array with size of PRIMES_LIMIT representing
    // all possible prime numbers before n.
    let mut array = [true; PRIME_LIMIT as usize];

    let n_root = sqrt_heron(n);

    // For each prime found set false for all your
    // multiples.
    for i in 2..n_root {
        if array[i as usize] == true {
            let mut j = i * i;

            while j < n {
                array[j as usize] = false;
                j = j + i;
            }
        }
    }

    let mut primes = [0 as u32; PRIMES_LENGHT];
    let mut head: usize = 0;

    // Get the last primes from array.
    for i in (0..n).rev() {
        if array[i as usize] == true {
            primes[head] = i;
            head += 1;
        }
        if head == PRIMES_LENGHT {
            break;
        }
    }

    return primes;
}

///
/// * Description *
///
/// Read the buffer handling semaphores
/// for avoid Race Condition.
///
fn read(sem: &[i32]) -> u32 {
    // Number of threads waiting.
    static mut READCOUNT: u32 = 0;

    let prime;

    // This semaphore ensures the writer's priority,
    // preveting that a writer and a reader are
    // waiting in the same semaphore.
    pm::semop(sem[3] as u32, SEMAPHORE_DOWN);

    // Block the reader if there is some writer
    // waiting for access buffer.
    pm::semop(sem[0] as u32, SEMAPHORE_DOWN);

    // Control access to the READCOUNT variable.
    pm::semop(sem[1] as u32, SEMAPHORE_DOWN);

    // Increment the reader count on the buffer.
    unsafe {
        READCOUNT = READCOUNT + 1;
        if READCOUNT == 1 {
            // Lock buffer for writers.
            pm::semop(sem[4] as u32, SEMAPHORE_DOWN);
        }
    }

    pm::semop(sem[1] as u32, SEMAPHORE_UP);
    pm::semop(sem[0] as u32, SEMAPHORE_UP);
    pm::semop(sem[3] as u32, SEMAPHORE_UP);

    let mut n = 0;
    prandom(&mut n);

    // Get a prime in a random position in buffer.
    unsafe {
        n = n % BUFFER_LENGHT as u32;
        prime = BUFFER[n as usize];
    }

    // Control access in the READCOUNT variable.
    pm::semop(sem[1] as u32, SEMAPHORE_DOWN);

    // Decrement the reader count on the buffer.
    unsafe {
        READCOUNT = READCOUNT - 1;
        if READCOUNT == 0 {
            // Unlock buffer for writers.
            pm::semop(sem[4] as u32, SEMAPHORE_UP);
        }
    }

    pm::semop(sem[1] as u32, SEMAPHORE_UP);

    return prime;
}

///
/// * Description *
///
/// Write in the buffer handling semaphores
/// for avoid Race Condition.
///
fn write(sem: &[i32], primes: [u32; PRIMES_LENGHT]) {
    // Number of threads accesing the buffer.
    static mut WRITECOUNT: u32 = 0;

    // Most old primes/positions in buffer.
    static mut OLD_POS: usize = 0;

    // Control access to the WRITECOUNT variable.
    pm::semop(sem[2] as u32, SEMAPHORE_DOWN);

    unsafe {
        WRITECOUNT = WRITECOUNT + 1;
        if WRITECOUNT == 1 {
            //Block the writer if there is some reader
            //in buffer.
            pm::semop(sem[0] as u32, SEMAPHORE_DOWN);
        }
    }

    pm::semop(sem[2] as u32, SEMAPHORE_UP);

    // Lock buffer.
    pm::semop(sem[4] as u32, SEMAPHORE_DOWN);

    unsafe {
        // Push new primes in buffer.
        for prime in primes {
            BUFFER[OLD_POS] = prime;
            OLD_POS = (OLD_POS + 1) % BUFFER_LENGHT;
        }
    }

    // Unlock buffer
    pm::semop(sem[4] as u32, SEMAPHORE_UP);

    // Access Write counter variable.
    pm::semop(sem[2] as u32, SEMAPHORE_DOWN);

    unsafe {
        // Decrement the writer count on the buffer.
        WRITECOUNT = WRITECOUNT - 1;
        if WRITECOUNT == 0 {
            pm::semop(sem[0] as u32, SEMAPHORE_UP);
        }
    }

    // Exit shared region.
    pm::semop(sem[2] as u32, SEMAPHORE_UP);
}

///
/// * Description *
///
/// Thread reader, read 2 primes from the buffer
/// and create RSA encryptation keys, private and
/// public.
///
fn reader(arg: *mut ffi::c_void) -> *mut ffi::c_void {
    // Cast c_void in i32 semaphore array.
    let sem: &mut [i32];
    unsafe {
        sem = slice::from_raw_parts_mut(arg as *mut i32, SEMAPHORE_TOTAL);
    }

    let mut count = OP_READER;

    // Reader loop.
    while count > 0 {
        // p and q are two primes that are part of the private key.
        let p = read(sem);
        let q = read(sem);

        // n is a part of the public key.
        let _n = p * q;

        // TODO - finish the rsa implementation.

        count -= 1;
    }

    return core::ptr::null_mut();
}

///
/// * Description *
///
/// Thread writer, find primes and put them in
/// the buffer.
///
fn writer(arg: *mut ffi::c_void) -> *mut ffi::c_void {
    // Cast c_void in i32 semaphore array.
    let sem: &mut [i32];
    unsafe {
        sem = slice::from_raw_parts_mut(arg as *mut i32, SEMAPHORE_TOTAL);
    }

    // Counts writer operations
    let mut count = OP_WRITER;

    while count > 0 {
        // Array with primes.
        let primes: [u32; PRIMES_LENGHT];

        let mut n = 0;

        // Generates a rand number.
        prandom(&mut n);

        // Find primes up to n.
        primes = find_primes(n % PRIME_LIMIT);
        write(sem, primes);

        count -= 1;
    }

    return core::ptr::null_mut();
}

///
/// ** Description **
///
/// Init a semaphore using a parameter key, set
/// start value 1 and return semaphore id.
///
fn init_semaphore(key: u32) -> i32 {
    let semid = pm::semget(key);
    if semid < 0 {
        return -1;
    }

    let ret = pm::semctl(semid as u32, SET_COUNT, 1);
    if ret < 0 {
        return -1;
    }

    return semid;
}

///
/// **Description**
///
/// Executes problem Readers and Writers
///
/// Readers and writers are a classic concurrency problem
/// where there are two entities accessing the buffer, the readers
/// and the writers. Readers can access the buffer together,
/// but they can only read. Writers have exclusive access to the
/// buffer, when a writer is accessing the buffer nobody can
/// access. In this implementation, writers have more priority than
/// readers. Furthermore, the writer threads find primes
/// and write in the buffer, the readers get the primes from
/// buffer and generate public and private keys using
/// RSA encryption algorithm.
///
pub fn readerswriters() -> bool {
    // Create semaphores array.
    let mut semaphores = [-1 as i32; SEMAPHORE_TOTAL];

    // Create random keys for buffer protector semaphores.
    let keys: [u32; 5] = [127661, 131673, 137677, 139683, 131621];

    // Get and init buffer protector semaphores.
    for i in 0..5 {
        semaphores[i] = init_semaphore(keys[i]);
        if semaphores[i] < 0 {
            return false;
        }
    }

    // Init prandom() semaphore.
    if init_semaphore(PRANDOM_KEY) < 0 {
        return false;
    }

    // Creates threads array.
    let mut readers = [-1 as Tid; READERS_TOTAL];
    let mut writers = [-1 as Tid; WRITERS_TOTAL];

    // Creates a pointer to semaphores array.
    let ptr_semaphores: *mut ffi::c_void =
        (&mut semaphores as *mut i32) as *mut ffi::c_void;

    // Creates readers threads.
    for i in 0..READERS_TOTAL {
        // Creates a reader thread.
        readers[i] = pm::thread_create(reader, ptr_semaphores);
        if readers[i] < 0 {
            nanvix::log!("Failed to create thread reader {}", i);
            return false;
        }
    }

    // Creates writers threads.
    for i in 0..WRITERS_TOTAL {
        // Creates a pointer to semaphores array.
        let p_semaphores: *mut ffi::c_void =
            &mut semaphores as *mut i32 as *mut ffi::c_void;

        // Creates a writer thread.
        writers[i] = pm::thread_create(writer, p_semaphores);
        if writers[i] < 0 {
            nanvix::log!("Failed to create thread writer {}", i);
            return false;
        }
    }

    let mut global_ret: *mut ffi::c_void = core::ptr::null_mut();

    // Join writers threads.
    for w in writers {
        pm::thread_join(w, &mut global_ret);
    }

    // Join readers threads.
    for r in readers {
        pm::thread_join(r, &mut global_ret);
    }

    return true;
}

//==============================================================================
// Public Standalone Functions
//==============================================================================

/// * Description *
///
/// Run readers and writers test.
///
pub fn test() {
    crate::test!(readerswriters());
}
