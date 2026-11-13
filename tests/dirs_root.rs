//! A test run rooted elsewhere must leave the real data folder alone.

use rune::dirs::{AppDirs, ROOT_ENV};
use typr::store::{self, APP_ID, database_path};

#[test]
fn a_rooted_run_writes_nothing_to_the_real_data_folder() {
    let root = tempfile::tempdir().expect("temp root");
    let real = AppDirs::new(APP_ID).map(|dirs| database_path(&dirs));

    // SAFETY: this test binary is one process with one test in it.
    unsafe { std::env::set_var(ROOT_ENV, root.path()) };

    let dirs = AppDirs::new(APP_ID).expect("rooted dirs");
    assert!(dirs.data().starts_with(root.path()), "{} is under the root", dirs.data().display());

    {
        let db = store::open_from_environment().expect("open through the environment");
        drop(db);
    }

    assert!(database_path(&dirs).exists(), "the library was created under the root");
    if let Some(real) = real {
        assert!(!real.exists(), "{} must not be created", real.display());
    }
}
