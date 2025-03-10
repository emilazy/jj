// Copyright 2023 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;

use crate::common::CommandOutput;
use crate::common::TestEnvironment;

fn create_commit(test_env: &TestEnvironment, repo_path: &Path, name: &str, parents: &[&str]) {
    let parents = match parents {
        [] => &["root()"],
        parents => parents,
    };
    test_env
        .run_jj_with(|cmd| {
            cmd.current_dir(repo_path)
                .args(["new", "-m", name])
                .args(parents)
        })
        .success();
    std::fs::write(repo_path.join(name), format!("{name}\n")).unwrap();
    test_env
        .run_jj_in(repo_path, ["bookmark", "create", "-r@", name])
        .success();
}

#[test]
fn test_basics() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    create_commit(&test_env, &repo_path, "a", &[]);
    create_commit(&test_env, &repo_path, "b", &["a"]);
    create_commit(&test_env, &repo_path, "c", &[]);
    create_commit(&test_env, &repo_path, "d", &["c"]);
    create_commit(&test_env, &repo_path, "e", &["a", "d"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    [znk] e
    ├─╮
    │ ○  [vru] d
    │ ○  [roy] c
    │ │ ○  [zsu] b
    ├───╯
    ○ │  [rlv] a
    ├─╯
    ◆  [zzz]
    [EOF]
    ");

    let output = test_env.run_jj_in(&repo_path, ["abandon", "--retain-bookmarks", "d"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit vruxwmqv b7c62f28 d | d
    Rebased 1 descendant commits onto parents of abandoned commits
    Working copy now at: znkkpsqq 11a2e10e e | e
    Parent commit      : rlvkpnrz 2443ea76 a | a
    Parent commit      : royxmykx fe2e8e8b c d | c
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    [znk] e
    ├─╮
    │ ○  [roy] c d
    │ │ ○  [zsu] b
    ├───╯
    ○ │  [rlv] a
    ├─╯
    ◆  [zzz]
    [EOF]
    ");

    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(
        &repo_path,
        ["abandon", "--retain-bookmarks"], /* abandons `e` */
    );
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit znkkpsqq 5557ece3 e | e
    Working copy now at: nkmrtpmo d4f8ea73 (empty) (no description set)
    Parent commit      : rlvkpnrz 2443ea76 a e?? | a
    Parent commit      : vruxwmqv b7c62f28 d e?? | d
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    [nkm]
    ├─╮
    │ ○  [vru] d e??
    │ ○  [roy] c
    │ │ ○  [zsu] b
    ├───╯
    ○ │  [rlv] a e??
    ├─╯
    ◆  [zzz]
    [EOF]
    ");

    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["abandon", "descendants(d)"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned the following commits:
      znkkpsqq 5557ece3 e | e
      vruxwmqv b7c62f28 d | d
    Deleted bookmarks: d, e
    Working copy now at: xtnwkqum fa4ee8e6 (empty) (no description set)
    Parent commit      : rlvkpnrz 2443ea76 a | a
    Parent commit      : royxmykx fe2e8e8b c | c
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    [xtn]
    ├─╮
    │ ○  [roy] c
    │ │ ○  [zsu] b
    ├───╯
    ○ │  [rlv] a
    ├─╯
    ◆  [zzz]
    [EOF]
    ");

    // Test abandoning the same commit twice directly
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["abandon", "-rb", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit zsuskuln 1394f625 b | b
    Deleted bookmarks: b
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    [znk] e
    ├─╮
    │ ○  [vru] d
    │ ○  [roy] c
    ○ │  [rlv] a
    ├─╯
    ◆  [zzz]
    [EOF]
    ");

    // Test abandoning the same commit twice indirectly
    test_env.run_jj_in(&repo_path, ["undo"]).success();
    let output = test_env.run_jj_in(&repo_path, ["abandon", "d::", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned the following commits:
      znkkpsqq 5557ece3 e | e
      vruxwmqv b7c62f28 d | d
    Deleted bookmarks: d, e
    Working copy now at: xlzxqlsl 14991aec (empty) (no description set)
    Parent commit      : rlvkpnrz 2443ea76 a | a
    Parent commit      : royxmykx fe2e8e8b c | c
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    [xlz]
    ├─╮
    │ ○  [roy] c
    │ │ ○  [zsu] b
    ├───╯
    ○ │  [rlv] a
    ├─╯
    ◆  [zzz]
    [EOF]
    ");

    let output = test_env.run_jj_in(&repo_path, ["abandon", "none()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    No revisions to abandon.
    [EOF]
    ");
}

// This behavior illustrates https://github.com/jj-vcs/jj/issues/2600.
// See also the corresponding test in `test_rebase_command`
#[test]
fn test_bug_2600() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    // We will not touch "nottherootcommit". See the
    // `test_bug_2600_rootcommit_special_case` for the one case where base being the
    // child of the root commit changes the expected behavior.
    create_commit(&test_env, &repo_path, "nottherootcommit", &[]);
    create_commit(&test_env, &repo_path, "base", &["nottherootcommit"]);
    create_commit(&test_env, &repo_path, "a", &["base"]);
    create_commit(&test_env, &repo_path, "b", &["base", "a"]);
    create_commit(&test_env, &repo_path, "c", &["b"]);

    // Test the setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  [znk] c
    ○    [vru] b
    ├─╮
    │ ○  [roy] a
    ├─╯
    ○  [zsu] base
    ○  [rlv] nottherootcommit
    ◆  [zzz]
    [EOF]
    ");
    let setup_opid = test_env.current_operation_id(&repo_path);

    test_env
        .run_jj_in(&repo_path, ["op", "restore", &setup_opid])
        .success();
    let output = test_env.run_jj_in(&repo_path, ["abandon", "base"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit zsuskuln 73c929fc base | base
    Deleted bookmarks: base
    Rebased 3 descendant commits onto parents of abandoned commits
    Working copy now at: znkkpsqq 86e31bec c | c
    Parent commit      : vruxwmqv fd6eb121 b | b
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    // Commits "a" and "b" should both have "nottherootcommit" as parent, and "b"
    // should keep "a" as second parent.
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  [znk] c
    ○    [vru] b
    ├─╮
    │ ○  [roy] a
    ├─╯
    ○  [rlv] nottherootcommit
    ◆  [zzz]
    [EOF]
    ");

    test_env
        .run_jj_in(&repo_path, ["op", "restore", &setup_opid])
        .success();
    let output = test_env.run_jj_in(&repo_path, ["abandon", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit royxmykx 98f3b9ba a | a
    Deleted bookmarks: a
    Rebased 2 descendant commits onto parents of abandoned commits
    Working copy now at: znkkpsqq 683b9435 c | c
    Parent commit      : vruxwmqv c10cb7b4 b | b
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    // Commit "b" should have "base" as parent. It should not have two parent
    // pointers to that commit even though it was a merge commit before we abandoned
    // "a".
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  [znk] c
    ○  [vru] b
    ○  [zsu] base
    ○  [rlv] nottherootcommit
    ◆  [zzz]
    [EOF]
    ");

    test_env
        .run_jj_in(&repo_path, ["op", "restore", &setup_opid])
        .success();
    let output = test_env.run_jj_in(&repo_path, ["abandon", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit vruxwmqv 8c0dced0 b | b
    Deleted bookmarks: b
    Rebased 1 descendant commits onto parents of abandoned commits
    Working copy now at: znkkpsqq 33a94991 c | c
    Parent commit      : zsuskuln 73c929fc base | base
    Parent commit      : royxmykx 98f3b9ba a | a
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    // Commit "c" should inherit the parents from the abndoned commit "b".
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @    [znk] c
    ├─╮
    │ ○  [roy] a
    ├─╯
    ○  [zsu] base
    ○  [rlv] nottherootcommit
    ◆  [zzz]
    [EOF]
    ");

    test_env
        .run_jj_in(&repo_path, ["op", "restore", &setup_opid])
        .success();
    // ========= Reminder of the setup ===========
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  [znk] c
    ○    [vru] b
    ├─╮
    │ ○  [roy] a
    ├─╯
    ○  [zsu] base
    ○  [rlv] nottherootcommit
    ◆  [zzz]
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["abandon", "--retain-bookmarks", "a", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned the following commits:
      vruxwmqv 8c0dced0 b | b
      royxmykx 98f3b9ba a | a
    Rebased 1 descendant commits onto parents of abandoned commits
    Working copy now at: znkkpsqq 84fac1f8 c | c
    Parent commit      : zsuskuln 73c929fc a b base | base
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    // Commit "c" should have "base" as parent. As when we abandoned "a", it should
    // not have two parent pointers to the same commit.
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  [znk] c
    ○  [zsu] a b base
    ○  [rlv] nottherootcommit
    ◆  [zzz]
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["bookmark", "list", "b"]);
    insta::assert_snapshot!(output, @r"
    b: zsuskuln 73c929fc base
    [EOF]
    ");
}

#[test]
fn test_bug_2600_rootcommit_special_case() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    // Set up like `test_bug_2600`, but without the `nottherootcommit` commit.
    create_commit(&test_env, &repo_path, "base", &[]);
    create_commit(&test_env, &repo_path, "a", &["base"]);
    create_commit(&test_env, &repo_path, "b", &["base", "a"]);
    create_commit(&test_env, &repo_path, "c", &["b"]);

    // Setup
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path), @r"
    @  [vru] c
    ○    [roy] b
    ├─╮
    │ ○  [zsu] a
    ├─╯
    ○  [rlv] base
    ◆  [zzz]
    [EOF]
    ");

    // Now, the test
    let output = test_env.run_jj_in(&repo_path, ["abandon", "base"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: The Git backend does not support creating merge commits with the root commit as one of the parents.
    [EOF]
    [exit status: 1]
    ");
}

#[test]
fn test_double_abandon() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    create_commit(&test_env, &repo_path, "a", &[]);
    // Test the setup
    insta::assert_snapshot!(
    test_env.run_jj_in(&repo_path, ["log", "--no-graph", "-r", "a"]), @r"
    rlvkpnrz test.user@example.com 2001-02-03 08:05:09 a 2443ea76
    a
    [EOF]
    ");

    let commit_id = test_env
        .run_jj_in(
            &repo_path,
            ["log", "--no-graph", "--color=never", "-T=commit_id", "-r=a"],
        )
        .success()
        .stdout
        .into_raw();

    let output = test_env.run_jj_in(&repo_path, ["abandon", &commit_id]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit rlvkpnrz 2443ea76 a | a
    Deleted bookmarks: a
    Working copy now at: royxmykx f37b4afd (empty) (no description set)
    Parent commit      : zzzzzzzz 00000000 (empty) (no description set)
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["abandon", &commit_id]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit rlvkpnrz hidden 2443ea76 a
    Nothing changed.
    [EOF]
    ");
}

#[test]
fn test_abandon_restore_descendants() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let repo_path = test_env.env_root().join("repo");

    std::fs::write(repo_path.join("file"), "foo\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    std::fs::write(repo_path.join("file"), "bar\n").unwrap();
    test_env.run_jj_in(&repo_path, ["new"]).success();
    std::fs::write(repo_path.join("file"), "baz\n").unwrap();

    // Remove the commit containing "bar"
    let output = test_env.run_jj_in(&repo_path, ["abandon", "-r@-", "--restore-descendants"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Abandoned commit rlvkpnrz 225adef1 (no description set)
    Rebased 1 descendant commits (while preserving their content) onto parents of abandoned commits
    Working copy now at: kkmpptxz a734deb0 (no description set)
    Parent commit      : qpvuntsm 485d52a9 (no description set)
    [EOF]
    ");
    let output = test_env.run_jj_in(&repo_path, ["diff", "--git"]);
    insta::assert_snapshot!(output, @r"
    diff --git a/file b/file
    index 257cc5642c..76018072e0 100644
    --- a/file
    +++ b/file
    @@ -1,1 +1,1 @@
    -foo
    +baz
    [EOF]
    ");
}

#[must_use]
fn get_log_output(test_env: &TestEnvironment, repo_path: &Path) -> CommandOutput {
    let template = r#"separate(" ", "[" ++ change_id.short(3) ++ "]", bookmarks)"#;
    test_env.run_jj_in(repo_path, ["log", "-T", template])
}
