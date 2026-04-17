use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

fn main() {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(
            ProcessRefreshKind::nothing().with_memory(),
        ),
    );
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_memory(),
    );

    let mut procs: Vec<_> = sys.processes().values()
        .filter(|p| p.thread_kind().is_none())
        .collect();
    procs.sort_by_key(|p| std::cmp::Reverse(p.memory()));
    
    for p in procs.iter().take(20) {
        println!("PID: {}, Name: {}, RSS: {}", p.pid(), p.name().to_string_lossy(), p.memory());
    }
}
