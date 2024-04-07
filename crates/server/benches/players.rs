use std::{net::SocketAddr, sync::atomic::AtomicU16};

use divan::Bencher;
use libc::{getrlimit, setrlimit, RLIMIT_NOFILE};
use rust_mc_bot::{Address, BotManager};
use server::Game;

static PORT: AtomicU16 = AtomicU16::new(25565);

fn adjust_file_limits() {
    unsafe {
        let mut limits = libc::rlimit {
            rlim_cur: 0, // Initialize soft limit to 0
            rlim_max: 0, // Initialize hard limit to 0
        };

        if getrlimit(RLIMIT_NOFILE, &mut limits) == 0 {
            println!("Current soft limit: {}", limits.rlim_cur);
            println!("Current hard limit: {}", limits.rlim_max);
        } else {
            eprintln!("Failed to get the maximum number of open file descriptors");
        }

        limits.rlim_cur = limits.rlim_max;
        println!("Setting soft limit to: {}", limits.rlim_cur);

        if setrlimit(RLIMIT_NOFILE, &limits) != 0 {
            eprintln!("Failed to set the maximum number of open file descriptors");
        }
    }
}

fn main() {
    // get current limit

    adjust_file_limits();

    divan::main();
}

const PLAYER_COUNTS: &[u32] = &[1, 2, 4, 8, 16, 32, 64, 128, 256];

#[divan::bench(
    args = PLAYER_COUNTS,
)]
fn n_bots_moving(bencher: Bencher, player_count: u32) {
    let port = PORT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let mut game = Game::init(addr).unwrap();

    let addrs = Address::TCP(addr);
    let mut bot_manager = BotManager::create(player_count, addrs, 0, 1).unwrap();

    loop {
        game.tick();
        bot_manager.tick();

        if game
            .shared()
            .player_count
            .load(std::sync::atomic::Ordering::Relaxed)
            == player_count
        {
            break;
        }
    }

    // we have completed the login sequence for all bots, now we can start benchmarking

    bencher.bench_local(|| {
        game.tick();
        bot_manager.tick();
    });

    game.shutdown();
}