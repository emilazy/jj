// Copyright 2022 The Jujutsu Authors
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

use itertools::Itertools;
use regex::Regex;
use testutils::git;

use crate::common::get_stdout_string;
use crate::common::CommandOutput;
use crate::common::TestEnvironment;

#[test]
fn test_op_log() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "description 0"]);

    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(&stdout, @r"
    @  d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj describe -m 'description 0'
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");
    let op_log_lines = stdout.raw().lines().collect_vec();
    let add_workspace_id = op_log_lines[3].split(' ').nth(2).unwrap();

    // Can load the repo at a specific operation ID
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path, add_workspace_id), @r"
    @  230dd059e1b059aefc0da06a2e5a7dbf22362f22
    ◆  0000000000000000000000000000000000000000
    [EOF]
    ");
    // "@" resolves to the head operation
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path, "@"), @r"
    @  19611c995a342c01f525583e5fcafdd211f6d009
    ◆  0000000000000000000000000000000000000000
    [EOF]
    ");
    // "@-" resolves to the parent of the head operation
    insta::assert_snapshot!(get_log_output(&test_env, &repo_path, "@-"), @r"
    @  230dd059e1b059aefc0da06a2e5a7dbf22362f22
    ◆  0000000000000000000000000000000000000000
    [EOF]
    ");
    insta::assert_snapshot!(
        test_env.run_jj_in(&repo_path, ["log", "--at-op", "@---"]), @r#"
    ------- stderr -------
    Error: The "@---" expression resolved to no operations
    [EOF]
    [exit status: 1]
    "#);

    // We get a reasonable message if an invalid operation ID is specified
    insta::assert_snapshot!(test_env.run_jj_in(&repo_path, ["log", "--at-op", "foo"]), @r#"
    ------- stderr -------
    Error: Operation ID "foo" is not a valid hexadecimal prefix
    [EOF]
    [exit status: 1]
    "#);

    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "--op-diff"]);
    insta::assert_snapshot!(&stdout, @r"
    @  d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj describe -m 'description 0'
    │
    │  Changed commits:
    │  ○  + qpvuntsm 19611c99 (empty) description 0
    │     - qpvuntsm hidden 230dd059 (empty) (no description set)
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    │
    │  Changed commits:
    │  ○  + qpvuntsm 230dd059 (empty) (no description set)
    ○  000000000000 root()
    [EOF]
    ");

    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "description 1"]);
    test_env.jj_cmd_ok(
        &repo_path,
        &[
            "describe",
            "-m",
            "description 2",
            "--at-op",
            add_workspace_id,
        ],
    );
    insta::assert_snapshot!(test_env.run_jj_in(&repo_path, ["log", "--at-op", "@-"]), @r#"
    ------- stderr -------
    Error: The "@" expression resolved to more than one operation
    Hint: Try specifying one of the operations by ID: fd29e648380b, 3e8ef7115a0c
    [EOF]
    [exit status: 1]
    "#);
}

#[test]
fn test_op_log_with_custom_symbols() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "description 0"]);

    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &[
            "op",
            "log",
            "--config=templates.op_log_node='if(current_operation, \"$\", if(root, \"┴\", \"┝\"))'",
        ],
    );
    insta::assert_snapshot!(&stdout, @r"
    $  d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj describe -m 'description 0'
    ┝  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ┴  000000000000 root()
    [EOF]
    ");
}

#[test]
fn test_op_log_with_no_template() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    let output = test_env.run_jj_in(&repo_path, ["op", "log", "-T"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    error: a value is required for '--template <TEMPLATE>' but none was supplied

    For more information, try '--help'.
    Hint: The following template aliases are defined:
    - builtin_log_comfortable
    - builtin_log_compact
    - builtin_log_compact_full_description
    - builtin_log_detailed
    - builtin_log_node
    - builtin_log_node_ascii
    - builtin_log_oneline
    - builtin_op_log_comfortable
    - builtin_op_log_compact
    - builtin_op_log_node
    - builtin_op_log_node_ascii
    - builtin_op_log_oneline
    - commit_summary_separator
    - description_placeholder
    - email_placeholder
    - name_placeholder
    [EOF]
    [exit status: 2]
    ");
}

#[test]
fn test_op_log_limit() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "-Tdescription", "--limit=1"]);
    insta::assert_snapshot!(stdout, @r"
    @  add workspace 'default'
    [EOF]
    ");
}

#[test]
fn test_op_log_no_graph() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    let stdout =
        test_env.jj_cmd_success(&repo_path, &["op", "log", "--no-graph", "--color=always"]);
    insta::assert_snapshot!(stdout, @r"
    [1m[38;5;12meac759b9ab75[39m [38;5;3mtest-username@host.example.com[39m [38;5;14m2001-02-03 04:05:07.000 +07:00[39m - [38;5;14m2001-02-03 04:05:07.000 +07:00[39m[0m
    [1madd workspace 'default'[0m
    [38;5;4m000000000000[39m [38;5;2mroot()[39m
    [EOF]
    ");

    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "--op-diff", "--no-graph"]);
    insta::assert_snapshot!(&stdout, @r"
    eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    add workspace 'default'

    Changed commits:
    + qpvuntsm 230dd059 (empty) (no description set)
    000000000000 root()
    [EOF]
    ");
}

#[test]
fn test_op_log_reversed() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "description 0"]);

    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "--reversed"]);
    insta::assert_snapshot!(&stdout, @r"
    ○  000000000000 root()
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    @  d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
       describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
       args: jj describe -m 'description 0'
    [EOF]
    ");

    test_env.jj_cmd_ok(
        &repo_path,
        &["describe", "-m", "description 1", "--at-op", "@-"],
    );

    // Should be able to display log with fork and branch points
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "log", "--reversed"]);
    insta::assert_snapshot!(&stderr, @r"
    Concurrent modification detected, resolving automatically.
    [EOF]
    ");
    insta::assert_snapshot!(&stdout, @r"
    ○  000000000000 root()
    ○    eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    ├─╮  add workspace 'default'
    │ ○  8e3e726be123 test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │ │  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │ │  args: jj describe -m 'description 1' --at-op @-
    ○ │  d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    ├─╯  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │    args: jj describe -m 'description 0'
    @  e4538ffdc13d test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
       reconcile divergent operations
       args: jj op log --reversed
    [EOF]
    ");

    // Should work correctly with `--no-graph`
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "--reversed", "--no-graph"]);
    insta::assert_snapshot!(&stdout, @r"
    000000000000 root()
    eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    add workspace 'default'
    8e3e726be123 test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    args: jj describe -m 'description 1' --at-op @-
    d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    args: jj describe -m 'description 0'
    e4538ffdc13d test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    reconcile divergent operations
    args: jj op log --reversed
    [EOF]
    ");

    // Should work correctly with `--limit`
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "--reversed", "--limit=3"]);
    insta::assert_snapshot!(stdout, @r"
    ○  8e3e726be123 test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj describe -m 'description 1' --at-op @-
    │ ○  d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    ├─╯  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │    args: jj describe -m 'description 0'
    @  e4538ffdc13d test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
       reconcile divergent operations
       args: jj op log --reversed
    [EOF]
    ");

    // Should work correctly with `--limit` and `--no-graph`
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "log", "--reversed", "--limit=2", "--no-graph"],
    );
    insta::assert_snapshot!(stdout, @r"
    d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    args: jj describe -m 'description 0'
    e4538ffdc13d test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    reconcile divergent operations
    args: jj op log --reversed
    [EOF]
    ");
}

#[test]
fn test_op_log_no_graph_null_terminated() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "message1"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "message2"]);

    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &[
            "op",
            "log",
            "--no-graph",
            "--template",
            r#"id.short(4) ++ "\0""#,
        ],
    );
    insta::assert_debug_snapshot!(stdout.normalized(), @r#""ef17\0f412\0eac7\00000\0""#);
}

#[test]
fn test_op_log_template() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    let render = |template| test_env.jj_cmd_success(&repo_path, &["op", "log", "-T", template]);

    insta::assert_snapshot!(render(r#"id ++ "\n""#), @r"
    @  eac759b9ab75793fd3da96e60939fb48f2cd2b2a9c1f13ffe723cf620f3005b8d3e7e923634a07ea39513e4f2f360c87b9ad5d331cf90d7a844864b83b72eba1
    ○  00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
    [EOF]
    ");
    insta::assert_snapshot!(
        render(r#"separate(" ", id.short(5), current_operation, user,
                                time.start(), time.end(), time.duration()) ++ "\n""#), @r"
    @  eac75 true test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 2001-02-03 04:05:07.000 +07:00 less than a microsecond
    ○  00000 false @ 1970-01-01 00:00:00.000 +00:00 1970-01-01 00:00:00.000 +00:00 less than a microsecond
    [EOF]
    ");

    // Negative length shouldn't cause panic.
    insta::assert_snapshot!(render(r#"id.short(-1) ++ "|""#), @r"
    @  <Error: out of range integral type conversion attempted>|
    ○  <Error: out of range integral type conversion attempted>|
    [EOF]
    ");

    // Test the default template, i.e. with relative start time and duration. We
    // don't generally use that template because it depends on the current time,
    // so we need to reset the time range format here.
    test_env.add_config(
        r#"
[template-aliases]
'format_time_range(time_range)' = 'time_range.end().ago() ++ ", lasted " ++ time_range.duration()'
        "#,
    );
    let regex = Regex::new(r"\d\d years").unwrap();
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(regex.replace_all(stdout.raw(), "NN years"), @r#"
    @  eac759b9ab75 test-username@host.example.com NN years ago, lasted less than a microsecond
    │  add workspace 'default'
    ○  000000000000 root()
    "#);
}

#[test]
fn test_op_log_builtin_templates() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    // Render without graph to test line ending
    let render = |template| {
        test_env.jj_cmd_success(&repo_path, &["op", "log", "-T", template, "--no-graph"])
    };
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "description 0"]);

    insta::assert_snapshot!(render(r#"builtin_op_log_compact"#), @r#"
    d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    args: jj describe -m 'description 0'
    eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    add workspace 'default'
    000000000000 root()
    [EOF]
    "#);

    insta::assert_snapshot!(render(r#"builtin_op_log_comfortable"#), @r#"
    d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    args: jj describe -m 'description 0'

    eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    add workspace 'default'

    000000000000 root()

    [EOF]
    "#);

    insta::assert_snapshot!(render(r#"builtin_op_log_oneline"#), @r"
    d009cfc04993 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00 describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22 args: jj describe -m 'description 0'
    eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00 add workspace 'default'
    000000000000 root()
    [EOF]
    ");
}

#[test]
fn test_op_log_word_wrap() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    std::fs::write(repo_path.join("file1"), "foo\n".repeat(100)).unwrap();
    test_env.jj_cmd_ok(&repo_path, &["debug", "snapshot"]);

    let render = |args: &[&str], columns: u32, word_wrap: bool| {
        let mut args = args.to_vec();
        if word_wrap {
            args.push("--config=ui.log-word-wrap=true");
        }
        let assert = test_env
            .jj_cmd(&repo_path, &args)
            .env("COLUMNS", columns.to_string())
            .assert()
            .success()
            .stderr("");
        get_stdout_string(&assert)
    };

    // ui.log-word-wrap option works
    insta::assert_snapshot!(render(&["op", "log"], 40, false), @r#"
    @  b7cd3d0069f6 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  snapshot working copy
    │  args: jj debug snapshot
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    "#);
    insta::assert_snapshot!(render(&["op", "log"], 40, true), @r#"
    @  b7cd3d0069f6
    │  test-username@host.example.com
    │  2001-02-03 04:05:08.000 +07:00 -
    │  2001-02-03 04:05:08.000 +07:00
    │  snapshot working copy
    │  args: jj debug snapshot
    ○  eac759b9ab75
    │  test-username@host.example.com
    │  2001-02-03 04:05:07.000 +07:00 -
    │  2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    "#);

    // Nested graph should be wrapped
    insta::assert_snapshot!(render(&["op", "log", "--op-diff"], 40, true), @r#"
    @  b7cd3d0069f6
    │  test-username@host.example.com
    │  2001-02-03 04:05:08.000 +07:00 -
    │  2001-02-03 04:05:08.000 +07:00
    │  snapshot working copy
    │  args: jj debug snapshot
    │
    │  Changed commits:
    │  ○  + qpvuntsm e292def1 (no
    │     description set)
    │     - qpvuntsm hidden 230dd059 (empty)
    │     (no description set)
    ○  eac759b9ab75
    │  test-username@host.example.com
    │  2001-02-03 04:05:07.000 +07:00 -
    │  2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    │
    │  Changed commits:
    │  ○  + qpvuntsm 230dd059 (empty) (no
    │     description set)
    ○  000000000000 root()
    "#);

    // Nested diff stat shouldn't exceed the terminal width
    insta::assert_snapshot!(render(&["op", "log", "-n1", "--stat"], 40, true), @r#"
    @  b7cd3d0069f6
    │  test-username@host.example.com
    │  2001-02-03 04:05:08.000 +07:00 -
    │  2001-02-03 04:05:08.000 +07:00
    │  snapshot working copy
    │  args: jj debug snapshot
    │
    │  Changed commits:
    │  ○  + qpvuntsm e292def1 (no
    │     description set)
    │     - qpvuntsm hidden 230dd059 (empty)
    │     (no description set)
    │     file1 | 100 +++++++++++++++++++
    │     1 file changed, 100 insertions(+), 0 deletions(-)
    "#);
    insta::assert_snapshot!(render(&["op", "log", "-n1", "--no-graph", "--stat"], 40, true), @r#"
    b7cd3d0069f6
    test-username@host.example.com
    2001-02-03 04:05:08.000 +07:00 -
    2001-02-03 04:05:08.000 +07:00
    snapshot working copy
    args: jj debug snapshot

    Changed commits:
    + qpvuntsm e292def1 (no description set)
    - qpvuntsm hidden 230dd059 (empty) (no
    description set)
    file1 | 100 +++++++++++++++++++++++++
    1 file changed, 100 insertions(+), 0 deletions(-)
    "#);

    // Nested graph widths should be subtracted from the term width
    let config = r#"templates.commit_summary='"0 1 2 3 4 5 6 7 8 9"'"#;
    insta::assert_snapshot!(
        render(&["op", "log", "-T''", "--op-diff", "-n1", "--config", config], 15, true), @r#"
    @
    │
    │  Changed
    │  commits:
    │  ○  + 0 1 2 3
    │     4 5 6 7 8
    │     9
    │     - 0 1 2 3
    │     4 5 6 7 8
    │     9
    "#);
}

#[test]
fn test_op_log_configurable() {
    let test_env = TestEnvironment::default();
    test_env.add_config(
        r#"operation.hostname = "my-hostname"
        operation.username = "my-username"
        "#,
    );
    test_env
        .jj_cmd(test_env.env_root(), &["git", "init", "repo"])
        .env_remove("JJ_OP_HOSTNAME")
        .env_remove("JJ_OP_USERNAME")
        .assert()
        .success();
    let repo_path = test_env.env_root().join("repo");

    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    assert!(stdout.raw().contains("my-username@my-hostname"));
}

#[test]
fn test_op_abandon_ancestors() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 1"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 2"]);
    insta::assert_snapshot!(test_env.jj_cmd_success(&repo_path, &["op", "log"]), @r"
    @  116edde65ded test-username@host.example.com 2001-02-03 04:05:09.000 +07:00 - 2001-02-03 04:05:09.000 +07:00
    │  commit 81a4ef3dd421f3184289df1c58bd3a16ea1e3d8e
    │  args: jj commit -m 'commit 2'
    ○  bee8c02a64bf test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj commit -m 'commit 1'
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");

    // Abandon old operations. The working-copy operation id should be updated.
    let (_stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "abandon", "..@-"]);
    insta::assert_snapshot!(stderr, @r"
    Abandoned 2 operations and reparented 1 descendant operations.
    [EOF]
    ");
    insta::assert_snapshot!(
        test_env.jj_cmd_success(&repo_path, &["debug", "local-working-copy", "--ignore-working-copy"]), @r#"
    Current operation: OperationId("8545e013752445fd845c84eb961dbfbce47e1deb628e4ef20df10f6dc9aae2ef9e47200b0fcc70ca51f050aede05d0fa6dd1db40e20ae740876775738a07d02e")
    Current tree: Merge(Resolved(TreeId("4b825dc642cb6eb9a060e54bf8d69288fbee4904")))
    [EOF]
    "#);
    insta::assert_snapshot!(test_env.jj_cmd_success(&repo_path, &["op", "log"]), @r"
    @  8545e0137524 test-username@host.example.com 2001-02-03 04:05:09.000 +07:00 - 2001-02-03 04:05:09.000 +07:00
    │  commit 81a4ef3dd421f3184289df1c58bd3a16ea1e3d8e
    │  args: jj commit -m 'commit 2'
    ○  000000000000 root()
    [EOF]
    ");

    // Abandon operation range.
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 3"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 4"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 5"]);
    let (_stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "abandon", "@---..@-"]);
    insta::assert_snapshot!(stderr, @r"
    Abandoned 2 operations and reparented 1 descendant operations.
    [EOF]
    ");
    insta::assert_snapshot!(test_env.jj_cmd_success(&repo_path, &["op", "log"]), @r"
    @  d92d0753399f test-username@host.example.com 2001-02-03 04:05:16.000 +07:00 - 2001-02-03 04:05:16.000 +07:00
    │  commit c5f7dd51add0046405055336ef443f882a0a8968
    │  args: jj commit -m 'commit 5'
    ○  8545e0137524 test-username@host.example.com 2001-02-03 04:05:09.000 +07:00 - 2001-02-03 04:05:09.000 +07:00
    │  commit 81a4ef3dd421f3184289df1c58bd3a16ea1e3d8e
    │  args: jj commit -m 'commit 2'
    ○  000000000000 root()
    [EOF]
    ");

    // Can't abandon the current operation.
    let output = test_env.run_jj_in(&repo_path, ["op", "abandon", "..@"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Cannot abandon the current operation d92d0753399f
    Hint: Run `jj undo` to revert the current operation, then use `jj op abandon`
    [EOF]
    [exit status: 1]
    ");

    // Can't create concurrent abandoned operations explicitly.
    let output = test_env.run_jj_in(&repo_path, ["op", "abandon", "--at-op=@-", "@"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: --at-op is not respected
    [EOF]
    [exit status: 2]
    ");

    // Abandon the current operation by undoing it first.
    test_env.jj_cmd_ok(&repo_path, &["undo"]);
    let (_stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "abandon", "@-"]);
    insta::assert_snapshot!(stderr, @r"
    Abandoned 1 operations and reparented 1 descendant operations.
    [EOF]
    ");
    insta::assert_snapshot!(
        test_env.jj_cmd_success(&repo_path, &["debug", "local-working-copy", "--ignore-working-copy"]), @r#"
    Current operation: OperationId("0699d720d0cecd80fb7d765c45955708c61b12feb1d7ed9ff2777ae719471f04ffed3c1dc24efdbf94bdb74426065d6fa9a4f0862a89db2c8c8e359eefc45462")
    Current tree: Merge(Resolved(TreeId("4b825dc642cb6eb9a060e54bf8d69288fbee4904")))
    [EOF]
    "#);
    insta::assert_snapshot!(test_env.jj_cmd_success(&repo_path, &["op", "log"]), @r"
    @  0699d720d0ce test-username@host.example.com 2001-02-03 04:05:21.000 +07:00 - 2001-02-03 04:05:21.000 +07:00
    │  undo operation d92d0753399f732e438bdd88fa7e5214cba2a310d120ec1714028a514c7116bcf04b4a0b26c04dbecf0a917f1d4c8eb05571b8816dd98b0502aaf321e92500b3
    │  args: jj undo
    ○  8545e0137524 test-username@host.example.com 2001-02-03 04:05:09.000 +07:00 - 2001-02-03 04:05:09.000 +07:00
    │  commit 81a4ef3dd421f3184289df1c58bd3a16ea1e3d8e
    │  args: jj commit -m 'commit 2'
    ○  000000000000 root()
    [EOF]
    ");

    // Abandon empty range.
    let (_stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "abandon", "@-..@-"]);
    insta::assert_snapshot!(stderr, @r"
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(test_env.jj_cmd_success(&repo_path, &["op", "log", "-n1"]), @r"
    @  0699d720d0ce test-username@host.example.com 2001-02-03 04:05:21.000 +07:00 - 2001-02-03 04:05:21.000 +07:00
    │  undo operation d92d0753399f732e438bdd88fa7e5214cba2a310d120ec1714028a514c7116bcf04b4a0b26c04dbecf0a917f1d4c8eb05571b8816dd98b0502aaf321e92500b3
    │  args: jj undo
    [EOF]
    ");
}

#[test]
fn test_op_abandon_without_updating_working_copy() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 1"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 2"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 3"]);

    // Abandon without updating the working copy.
    let (_stdout, stderr) = test_env.jj_cmd_ok(
        &repo_path,
        &["op", "abandon", "@-", "--ignore-working-copy"],
    );
    insta::assert_snapshot!(stderr, @r"
    Abandoned 1 operations and reparented 1 descendant operations.
    [EOF]
    ");
    insta::assert_snapshot!(
        test_env.jj_cmd_success(&repo_path, &["debug", "local-working-copy", "--ignore-working-copy"]), @r#"
    Current operation: OperationId("b0711a8ac91f5ac088cff9b57c9daf29dc61b1b4fedcbb9a07fe4c7f7da1e60e333c787eacf73d1e0544db048a4fe9c6c089991b4a67e25365c4f411fa8b489f")
    Current tree: Merge(Resolved(TreeId("4b825dc642cb6eb9a060e54bf8d69288fbee4904")))
    [EOF]
    "#);
    insta::assert_snapshot!(
        test_env.jj_cmd_success(&repo_path, &["op", "log", "-n1", "--ignore-working-copy"]), @r"
    @  0508a30825ed test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │  commit 220cb0b1b5d1c03cc0d351139d824598bb3c1967
    │  args: jj commit -m 'commit 3'
    [EOF]
    ");

    // The working-copy operation id isn't updated if it differs from the repo.
    // It could be updated if the tree matches, but there's no extra logic for
    // that.
    let (_stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "abandon", "@-"]);
    insta::assert_snapshot!(stderr, @r"
    Abandoned 1 operations and reparented 1 descendant operations.
    Warning: The working copy operation b0711a8ac91f is not updated because it differs from the repo 0508a30825ed.
    [EOF]
    ");
    insta::assert_snapshot!(
        test_env.jj_cmd_success(&repo_path, &["debug", "local-working-copy", "--ignore-working-copy"]), @r#"
    Current operation: OperationId("b0711a8ac91f5ac088cff9b57c9daf29dc61b1b4fedcbb9a07fe4c7f7da1e60e333c787eacf73d1e0544db048a4fe9c6c089991b4a67e25365c4f411fa8b489f")
    Current tree: Merge(Resolved(TreeId("4b825dc642cb6eb9a060e54bf8d69288fbee4904")))
    [EOF]
    "#);
    insta::assert_snapshot!(
        test_env.jj_cmd_success(&repo_path, &["op", "log", "-n1", "--ignore-working-copy"]), @r"
    @  2631d5576876 test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │  commit 220cb0b1b5d1c03cc0d351139d824598bb3c1967
    │  args: jj commit -m 'commit 3'
    [EOF]
    ");
}

#[test]
fn test_op_abandon_multiple_heads() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    // Create 1 base operation + 2 operations to be diverged.
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 1"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 2"]);
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "commit 3"]);
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "log", "--no-graph", r#"-Tid.short() ++ "\n""#],
    );
    let (head_op_id, prev_op_id) = stdout.raw().lines().next_tuple().unwrap();
    insta::assert_snapshot!(head_op_id, @"b0711a8ac91f");
    insta::assert_snapshot!(prev_op_id, @"116edde65ded");

    // Create 1 other concurrent operation.
    test_env.jj_cmd_ok(&repo_path, &["commit", "--at-op=@--", "-m", "commit 4"]);

    // Can't resolve operation relative to @.
    let output = test_env.run_jj_in(&repo_path, ["op", "abandon", "@-"]);
    insta::assert_snapshot!(output, @r#"
    ------- stderr -------
    Error: The "@" expression resolved to more than one operation
    Hint: Try specifying one of the operations by ID: b0711a8ac91f, 617923db9f7a
    [EOF]
    [exit status: 1]
    "#);
    let (_, other_head_op_id) = output.stderr.raw().trim_end().rsplit_once(", ").unwrap();
    insta::assert_snapshot!(other_head_op_id, @"617923db9f7a");
    assert_ne!(head_op_id, other_head_op_id);

    // Can't abandon one of the head operations.
    let output = test_env.run_jj_in(&repo_path, ["op", "abandon", head_op_id]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Cannot abandon the current operation b0711a8ac91f
    [EOF]
    [exit status: 1]
    ");

    // Can't abandon the other head operation.
    let output = test_env.run_jj_in(&repo_path, ["op", "abandon", other_head_op_id]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Cannot abandon the current operation 617923db9f7a
    [EOF]
    [exit status: 1]
    ");

    // Can abandon the operation which is not an ancestor of the other head.
    // This would crash if we attempted to remap the unchanged op in the op
    // heads store.
    let (_stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "abandon", prev_op_id]);
    insta::assert_snapshot!(stderr, @r"
    Abandoned 1 operations and reparented 2 descendant operations.
    [EOF]
    ");

    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(stdout, @r"
    @    7e65e7e27e34 test-username@host.example.com 2001-02-03 04:05:17.000 +07:00 - 2001-02-03 04:05:17.000 +07:00
    ├─╮  reconcile divergent operations
    │ │  args: jj op log
    ○ │  0508a30825ed test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │ │  commit 220cb0b1b5d1c03cc0d351139d824598bb3c1967
    │ │  args: jj commit -m 'commit 3'
    │ ○  617923db9f7a test-username@host.example.com 2001-02-03 04:05:12.000 +07:00 - 2001-02-03 04:05:12.000 +07:00
    ├─╯  commit 81a4ef3dd421f3184289df1c58bd3a16ea1e3d8e
    │    args: jj commit '--at-op=@--' -m 'commit 4'
    ○  bee8c02a64bf test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj commit -m 'commit 1'
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");
    insta::assert_snapshot!(stderr, @r"
    Concurrent modification detected, resolving automatically.
    [EOF]
    ");
}

#[test]
fn test_op_recover_from_bad_gc() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo", "--colocate"]);
    let repo_path = test_env.env_root().join("repo");
    let git_object_path = |hex: &str| {
        let (shard, file_name) = hex.split_at(2);
        let mut file_path = repo_path.clone();
        file_path.extend([".git", "objects", shard, file_name]);
        file_path
    };

    test_env.jj_cmd_ok(&repo_path, &["describe", "-m1"]);
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m2"]); // victim
    test_env.jj_cmd_ok(&repo_path, &["abandon"]); // break predecessors chain
    test_env.jj_cmd_ok(&repo_path, &["new", "-m3"]);
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m4"]);

    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "log", "--no-graph", r#"-Tid.short() ++ "\n""#],
    );
    let (head_op_id, _, _, bad_op_id) = stdout.raw().lines().next_tuple().unwrap();
    insta::assert_snapshot!(head_op_id, @"f999e12a5d8b");
    insta::assert_snapshot!(bad_op_id, @"e7377e6a642b");

    // Corrupt the repo by removing hidden but reachable commit object.
    let bad_commit_id = test_env
        .jj_cmd_success(
            &repo_path,
            &[
                "log",
                "--at-op",
                bad_op_id,
                "--no-graph",
                "-r@",
                "-Tcommit_id",
            ],
        )
        .into_raw();
    insta::assert_snapshot!(bad_commit_id, @"ddf84fc5e0dd314092b3dfb13e09e37fa7d04ef9");
    std::fs::remove_file(git_object_path(&bad_commit_id)).unwrap();

    // Do concurrent modification to make the situation even worse. At this
    // point, the index can be loaded, so this command succeeds.
    test_env.jj_cmd_ok(&repo_path, &["--at-op=@-", "describe", "-m4.1"]);

    let output = test_env.run_jj_in(&repo_path, ["--at-op", head_op_id, "debug", "reindex"]);
    insta::assert_snapshot!(output.strip_stderr_last_line(), @r"
    ------- stderr -------
    Internal error: Failed to index commits at operation e7377e6a642bae88039615ee159117d49688719e9d5ece9de8b0b42d7be7076904d2fa8381391f8289a0c3527405de81e8dd6504655311c69175c3681786dd3c
    Caused by:
    1: Object ddf84fc5e0dd314092b3dfb13e09e37fa7d04ef9 of type commit not found
    [EOF]
    [exit status: 255]
    ");

    // "op log" should still be usable.
    let (stdout, stderr) = test_env.jj_cmd_ok(
        &repo_path,
        &["op", "log", "--ignore-working-copy", "--at-op", head_op_id],
    );
    insta::assert_snapshot!(stdout, @r"
    @  f999e12a5d8b test-username@host.example.com 2001-02-03 04:05:12.000 +07:00 - 2001-02-03 04:05:12.000 +07:00
    │  describe commit 37bb762e5dc08073ec4323bdffc023a0f0cc901e
    │  args: jj describe -m4
    ○  fb75e6b1c70a test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    │  new empty commit
    │  args: jj new -m3
    ○  44d11f83204d test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │  abandon commit ddf84fc5e0dd314092b3dfb13e09e37fa7d04ef9
    │  args: jj abandon
    ○  e7377e6a642b test-username@host.example.com 2001-02-03 04:05:09.000 +07:00 - 2001-02-03 04:05:09.000 +07:00
    │  describe commit 8b64ddff700dc214dec05d915e85ac692233e6e3
    │  args: jj describe -m2
    ○  319610522e90 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj describe -m1
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");
    insta::assert_snapshot!(stderr, @"");

    // "op abandon" should work.
    let (_stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["op", "abandon", &format!("..{bad_op_id}")]);
    insta::assert_snapshot!(stderr, @r"
    Abandoned 3 operations and reparented 4 descendant operations.
    [EOF]
    ");

    // The repo should no longer be corrupt.
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["log"]);
    insta::assert_snapshot!(stdout, @r"
    @  mzvwutvl?? test.user@example.com 2001-02-03 08:05:12 6d868f04
    │  (empty) 4
    │ ○  mzvwutvl?? test.user@example.com 2001-02-03 08:05:15 dc2c6d52
    ├─╯  (empty) 4.1
    ○  zsuskuln test.user@example.com 2001-02-03 08:05:10 git_head() f652c321
    │  (empty) (no description set)
    ◆  zzzzzzzz root() 00000000
    [EOF]
    ");
    insta::assert_snapshot!(stderr, @r"
    Concurrent modification detected, resolving automatically.
    [EOF]
    ");
}

#[test]
fn test_op_summary_diff_template() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    // Tests in color (easier to read with `less -R`)
    test_env.jj_cmd_ok(&repo_path, &["new", "--no-edit", "-m=scratch"]);
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "undo", "--color=always"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Undid operation: [38;5;4mac20a4ff4791[39m ([38;5;6m2001-02-03 08:05:08[39m) new empty commit
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &[
            "op",
            "diff",
            "--from",
            "0000000",
            "--to",
            "@",
            "--color=always",
        ],
    );
    insta::assert_snapshot!(&stdout, @r"
    From operation: [38;5;4m000000000000[39m [38;5;2mroot()[39m
      To operation: [38;5;4me3792fce5b1f[39m ([38;5;6m2001-02-03 08:05:09[39m) undo operation ac20a4ff47914da9a2e43677b94455b86383bfb9227374d6531ecee85b9ff9230eeb96416a24bb27e7477aa18d50c01810e97c6a008b5c584224650846f4c05b

    Changed commits:
    ○  [38;5;2m+[39m [1m[38;5;5mq[0m[38;5;8mpvuntsm[39m [1m[38;5;4m2[0m[38;5;8m30dd059[39m [38;5;2m(empty)[39m [38;5;2m(no description set)[39m
    [EOF]
    ");

    // Tests with templates
    test_env.jj_cmd_ok(&repo_path, &["new", "--no-edit", "-m=scratch"]);
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "undo", "--color=debug"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Undid operation: [38;5;4m<<operation id short::2301f6e6ec31>>[39m<<operation:: (>>[38;5;6m<<operation time end local format::2001-02-03 08:05:11>>[39m<<operation::) >><<operation description first_line::new empty commit>>
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &[
            "op",
            "diff",
            "--from",
            "0000000",
            "--to",
            "@",
            "--color=debug",
        ],
    );
    insta::assert_snapshot!(&stdout, @r"
    From operation: [38;5;4m<<operation id short::000000000000>>[39m<<operation:: >>[38;5;2m<<operation root::root()>>[39m
      To operation: [38;5;4m<<operation id short::d208ae1b4e3c>>[39m<<operation:: (>>[38;5;6m<<operation time end local format::2001-02-03 08:05:12>>[39m<<operation::) >><<operation description first_line::undo operation 2301f6e6ec31931a9b0a594742d6035a44c05250d1707f7f8678e888b11a98773ef07bf0e8008a5bccddf7114da4a35d1a1b1f7efa37c1e6c80d6bdb8f0d7a90>>

    Changed commits:
    ○  [38;5;2m<<diff added::+>>[39m [1m[38;5;5m<<change_id shortest prefix::q>>[0m[38;5;8m<<change_id shortest rest::pvuntsm>>[39m [1m[38;5;4m<<commit_id shortest prefix::2>>[0m[38;5;8m<<commit_id shortest rest::30dd059>>[39m [38;5;2m<<empty::(empty)>>[39m [38;5;2m<<empty description placeholder::(no description set)>>[39m
    [EOF]
    ");
}

#[test]
fn test_op_diff() {
    let test_env = TestEnvironment::default();
    let git_repo_path = test_env.env_root().join("git-repo");
    let git_repo = init_bare_git_repo(&git_repo_path);
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "clone", "git-repo", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    // Overview of op log.
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(&stdout, @r"
    @  364d0a677b0c test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  check out git remote's default branch
    │  args: jj git clone git-repo repo
    ○  369ee2939177 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  fetch from git remote into empty repo
    │  args: jj git clone git-repo repo
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");

    // Diff between the same operation should be empty.
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "diff", "--from", "0000000", "--to", "0000000"],
    );
    insta::assert_snapshot!(&stdout, @r"
    From operation: 000000000000 root()
      To operation: 000000000000 root()
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "--from", "@", "--to", "@"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 364d0a677b0c (2001-02-03 08:05:07) check out git remote's default branch
      To operation: 364d0a677b0c (2001-02-03 08:05:07) check out git remote's default branch
    [EOF]
    ");

    // Diff from parent operation to latest operation.
    // `jj op diff --op @` should behave identically to `jj op diff --from
    // @- --to @` (if `@` is not a merge commit).
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "--from", "@-", "--to", "@"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 369ee2939177 (2001-02-03 08:05:07) fetch from git remote into empty repo
      To operation: 364d0a677b0c (2001-02-03 08:05:07) check out git remote's default branch

    Changed commits:
    ○  + sqpuoqvx c7b48fea (empty) (no description set)
    ○  - qpvuntsm hidden 230dd059 (empty) (no description set)

    Changed local bookmarks:
    bookmark-1:
    + pukowqtp 0cb7e07e bookmark-1 | Commit 1
    - (absent)

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked pukowqtp 0cb7e07e bookmark-1 | Commit 1
    - untracked pukowqtp 0cb7e07e bookmark-1 | Commit 1
    [EOF]
    ");
    let stdout_without_from_to = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    assert_eq!(stdout.raw(), stdout_without_from_to.raw());

    // Diff from root operation to latest operation
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "--from", "0000000"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 000000000000 root()
      To operation: 364d0a677b0c (2001-02-03 08:05:07) check out git remote's default branch

    Changed commits:
    ○  + sqpuoqvx c7b48fea (empty) (no description set)
    ○  + pukowqtp 0cb7e07e bookmark-1 | Commit 1
    ○  + rnnslrkn 4ff62539 bookmark-2@origin | Commit 2
    ○  + rnnkyono 11671e4c bookmark-3@origin | Commit 3

    Changed local bookmarks:
    bookmark-1:
    + pukowqtp 0cb7e07e bookmark-1 | Commit 1
    - (absent)

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked pukowqtp 0cb7e07e bookmark-1 | Commit 1
    - untracked (absent)
    bookmark-2@origin:
    + untracked rnnslrkn 4ff62539 bookmark-2@origin | Commit 2
    - untracked (absent)
    bookmark-3@origin:
    + untracked rnnkyono 11671e4c bookmark-3@origin | Commit 3
    - untracked (absent)
    [EOF]
    ");

    // Diff from latest operation to root operation
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "--to", "0000000"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 364d0a677b0c (2001-02-03 08:05:07) check out git remote's default branch
      To operation: 000000000000 root()

    Changed commits:
    ○  - sqpuoqvx hidden c7b48fea (empty) (no description set)
    ○  - pukowqtp hidden 0cb7e07e Commit 1
    ○  - rnnslrkn hidden 4ff62539 Commit 2
    ○  - rnnkyono hidden 11671e4c Commit 3

    Changed local bookmarks:
    bookmark-1:
    + (absent)
    - pukowqtp hidden 0cb7e07e Commit 1

    Changed remote bookmarks:
    bookmark-1@origin:
    + untracked (absent)
    - tracked pukowqtp hidden 0cb7e07e Commit 1
    bookmark-2@origin:
    + untracked (absent)
    - untracked rnnslrkn hidden 4ff62539 Commit 2
    bookmark-3@origin:
    + untracked (absent)
    - untracked rnnkyono hidden 11671e4c Commit 3
    [EOF]
    ");

    // Create a conflicted bookmark using a concurrent operation.
    test_env.jj_cmd_ok(
        &repo_path,
        &[
            "bookmark",
            "set",
            "bookmark-1",
            "-r",
            "bookmark-2@origin",
            "--at-op",
            "@-",
        ],
    );
    let (_, stderr) = test_env.jj_cmd_ok(&repo_path, &["log"]);
    insta::assert_snapshot!(&stderr, @r"
    Concurrent modification detected, resolving automatically.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(&stdout, @r"
    @    7f6887c5ae04 test-username@host.example.com 2001-02-03 04:05:16.000 +07:00 - 2001-02-03 04:05:16.000 +07:00
    ├─╮  reconcile divergent operations
    │ │  args: jj log
    ○ │  364d0a677b0c test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │ │  check out git remote's default branch
    │ │  args: jj git clone git-repo repo
    │ ○  ee092a3adf88 test-username@host.example.com 2001-02-03 04:05:15.000 +07:00 - 2001-02-03 04:05:15.000 +07:00
    ├─╯  point bookmark bookmark-1 to commit 4ff6253913375c6ebdddd8423c11df3b3f17e331
    │    args: jj bookmark set bookmark-1 -r bookmark-2@origin --at-op @-
    ○  369ee2939177 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  fetch from git remote into empty repo
    │  args: jj git clone git-repo repo
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");
    let op_log_lines = stdout.raw().lines().collect_vec();
    let op_id = op_log_lines[0].split(' ').nth(4).unwrap();
    let first_parent_id = op_log_lines[3].split(' ').nth(3).unwrap();
    let second_parent_id = op_log_lines[6].split(' ').nth(3).unwrap();

    // Diff between the first parent of the merge operation and the merge operation.
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "diff", "--from", first_parent_id, "--to", op_id],
    );
    insta::assert_snapshot!(&stdout, @r"
    From operation: 364d0a677b0c (2001-02-03 08:05:07) check out git remote's default branch
      To operation: 7f6887c5ae04 (2001-02-03 08:05:16) reconcile divergent operations

    Changed local bookmarks:
    bookmark-1:
    + (added) pukowqtp 0cb7e07e bookmark-1?? bookmark-1@origin | Commit 1
    + (added) rnnslrkn 4ff62539 bookmark-1?? bookmark-2@origin | Commit 2
    - pukowqtp 0cb7e07e bookmark-1?? bookmark-1@origin | Commit 1
    [EOF]
    ");

    // Diff between the second parent of the merge operation and the merge
    // operation.
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "diff", "--from", second_parent_id, "--to", op_id],
    );
    insta::assert_snapshot!(&stdout, @r"
    From operation: ee092a3adf88 (2001-02-03 08:05:15) point bookmark bookmark-1 to commit 4ff6253913375c6ebdddd8423c11df3b3f17e331
      To operation: 7f6887c5ae04 (2001-02-03 08:05:16) reconcile divergent operations

    Changed commits:
    ○  + sqpuoqvx c7b48fea (empty) (no description set)
    ○  - qpvuntsm hidden 230dd059 (empty) (no description set)

    Changed local bookmarks:
    bookmark-1:
    + (added) pukowqtp 0cb7e07e bookmark-1?? bookmark-1@origin | Commit 1
    + (added) rnnslrkn 4ff62539 bookmark-1?? bookmark-2@origin | Commit 2
    - rnnslrkn 4ff62539 bookmark-1?? bookmark-2@origin | Commit 2

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked pukowqtp 0cb7e07e bookmark-1?? bookmark-1@origin | Commit 1
    - untracked pukowqtp 0cb7e07e bookmark-1?? bookmark-1@origin | Commit 1
    [EOF]
    ");

    // Test fetching from git remote.
    modify_git_repo(git_repo);
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["git", "fetch"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    bookmark: bookmark-1@origin [updated] tracked
    bookmark: bookmark-2@origin [updated] untracked
    bookmark: bookmark-3@origin [deleted] untracked
    Abandoned 1 commits that are no longer reachable.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 7f6887c5ae04 (2001-02-03 08:05:16) reconcile divergent operations
      To operation: 0fda9dbbc995 (2001-02-03 08:05:20) fetch from git remote(s) origin

    Changed commits:
    ○  + kulxwnxm e1a239a5 bookmark-2@origin | Commit 5
    ○  + zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    ○  - rnnkyono hidden 11671e4c Commit 3

    Changed local bookmarks:
    bookmark-1:
    + (added) zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    + (added) rnnslrkn 4ff62539 bookmark-1?? | Commit 2
    - (added) pukowqtp 0cb7e07e Commit 1
    - (added) rnnslrkn 4ff62539 bookmark-1?? | Commit 2

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    - tracked pukowqtp 0cb7e07e Commit 1
    bookmark-2@origin:
    + untracked kulxwnxm e1a239a5 bookmark-2@origin | Commit 5
    - untracked rnnslrkn 4ff62539 bookmark-1?? | Commit 2
    bookmark-3@origin:
    + untracked (absent)
    - untracked rnnkyono hidden 11671e4c Commit 3
    [EOF]
    ");

    // Test creation of bookmark.
    let (stdout, stderr) = test_env.jj_cmd_ok(
        &repo_path,
        &[
            "bookmark",
            "create",
            "bookmark-2",
            "-r",
            "bookmark-2@origin",
        ],
    );
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Created 1 bookmarks pointing to kulxwnxm e1a239a5 bookmark-2 bookmark-2@origin | Commit 5
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 0fda9dbbc995 (2001-02-03 08:05:20) fetch from git remote(s) origin
      To operation: 505a09f5c0f0 (2001-02-03 08:05:22) create bookmark bookmark-2 pointing to commit e1a239a57eb15cefc5910198befbbbe2b43c47af

    Changed local bookmarks:
    bookmark-2:
    + kulxwnxm e1a239a5 bookmark-2 bookmark-2@origin | Commit 5
    - (absent)
    [EOF]
    ");

    // Test tracking of bookmark.
    let (stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["bookmark", "track", "bookmark-2@origin"]);
    insta::assert_snapshot!(&stdout, @r###"
     "###);
    insta::assert_snapshot!(&stderr, @r"
    Started tracking 1 remote bookmarks.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 505a09f5c0f0 (2001-02-03 08:05:22) create bookmark bookmark-2 pointing to commit e1a239a57eb15cefc5910198befbbbe2b43c47af
      To operation: e7d3f25689e1 (2001-02-03 08:05:24) track remote bookmark bookmark-2@origin

    Changed remote bookmarks:
    bookmark-2@origin:
    + tracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    - untracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    [EOF]
    ");

    // Test creation of new commit.
    // Test tracking of bookmark.
    let (stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["bookmark", "track", "bookmark-2@origin"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Warning: Remote bookmark already tracked: bookmark-2@origin
    Nothing changed.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 505a09f5c0f0 (2001-02-03 08:05:22) create bookmark bookmark-2 pointing to commit e1a239a57eb15cefc5910198befbbbe2b43c47af
      To operation: e7d3f25689e1 (2001-02-03 08:05:24) track remote bookmark bookmark-2@origin

    Changed remote bookmarks:
    bookmark-2@origin:
    + tracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    - untracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    [EOF]
    ");

    // Test creation of new commit.
    let (stdout, stderr) = test_env.jj_cmd_ok(
        &repo_path,
        &["new", "bookmark-1@origin", "-m", "new commit"],
    );
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Working copy now at: wvuyspvk fefb1e17 (empty) new commit
    Parent commit      : zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: e7d3f25689e1 (2001-02-03 08:05:24) track remote bookmark bookmark-2@origin
      To operation: b94b5ef70c8a (2001-02-03 08:05:28) new empty commit

    Changed commits:
    ○  + wvuyspvk fefb1e17 (empty) new commit
    ○  - sqpuoqvx hidden c7b48fea (empty) (no description set)
    [EOF]
    ");

    // Test updating of local bookmark.
    let (stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["bookmark", "set", "bookmark-1", "-r", "@"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Moved 1 bookmarks to wvuyspvk fefb1e17 bookmark-1* | (empty) new commit
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: b94b5ef70c8a (2001-02-03 08:05:28) new empty commit
      To operation: 26918495eee5 (2001-02-03 08:05:30) point bookmark bookmark-1 to commit fefb1e17c85328767a596c6dc3d9d604c024a02c

    Changed local bookmarks:
    bookmark-1:
    + wvuyspvk fefb1e17 bookmark-1* | (empty) new commit
    - (added) zkmtkqvo 0dee6313 bookmark-1@origin | Commit 4
    - (added) rnnslrkn 4ff62539 Commit 2
    [EOF]
    ");

    // Test deletion of local bookmark.
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["bookmark", "delete", "bookmark-2"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Deleted 1 bookmarks.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 26918495eee5 (2001-02-03 08:05:30) point bookmark bookmark-1 to commit fefb1e17c85328767a596c6dc3d9d604c024a02c
      To operation: 9969a6088fd3 (2001-02-03 08:05:32) delete bookmark bookmark-2

    Changed local bookmarks:
    bookmark-2:
    + (absent)
    - kulxwnxm e1a239a5 bookmark-2@origin | Commit 5
    [EOF]
    ");

    // Test pushing to Git remote.
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["git", "push", "--tracked"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Changes to push to origin:
      Move forward bookmark bookmark-1 from 0dee631320b1 to fefb1e17c853
      Delete bookmark bookmark-2 from e1a239a57eb1
    Warning: The working-copy commit in workspace 'default' became immutable, so a new commit has been created on top of it.
    Working copy now at: oupztwtk fe3ad088 (empty) (no description set)
    Parent commit      : wvuyspvk fefb1e17 bookmark-1 | (empty) new commit
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 9969a6088fd3 (2001-02-03 08:05:32) delete bookmark bookmark-2
      To operation: ff305cfe0aca (2001-02-03 08:05:34) push all tracked bookmarks to git remote origin

    Changed commits:
    ○  + oupztwtk fe3ad088 (empty) (no description set)

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked wvuyspvk fefb1e17 bookmark-1 | (empty) new commit
    - tracked zkmtkqvo 0dee6313 Commit 4
    bookmark-2@origin:
    + untracked (absent)
    - tracked kulxwnxm e1a239a5 Commit 5
    [EOF]
    ");
}

#[test]
fn test_op_diff_patch() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    // Update working copy with a single file and create new commit.
    std::fs::write(repo_path.join("file"), "a\n").unwrap();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["new"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Working copy now at: rlvkpnrz 56950632 (empty) (no description set)
    Parent commit      : qpvuntsm 6b1027d2 (no description set)
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "--op", "@-", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: eac759b9ab75 (2001-02-03 08:05:07) add workspace 'default'
      To operation: 187a5a9d8a22 (2001-02-03 08:05:08) snapshot working copy

    Changed commits:
    ○  + qpvuntsm 6b1027d2 (no description set)
       - qpvuntsm hidden 230dd059 (empty) (no description set)
       diff --git a/file b/file
       new file mode 100644
       index 0000000000..7898192261
       --- /dev/null
       +++ b/file
       @@ -0,0 +1,1 @@
       +a
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "--op", "@", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 187a5a9d8a22 (2001-02-03 08:05:08) snapshot working copy
      To operation: a7e535e73c4b (2001-02-03 08:05:08) new empty commit

    Changed commits:
    ○  + rlvkpnrz 56950632 (empty) (no description set)
    [EOF]
    ");

    // Squash the working copy commit.
    std::fs::write(repo_path.join("file"), "b\n").unwrap();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["squash"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Working copy now at: mzvwutvl 9f4fb57f (empty) (no description set)
    Parent commit      : qpvuntsm 2ac85fd1 (no description set)
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 15c3c5d0baf0 (2001-02-03 08:05:11) snapshot working copy
      To operation: 894c12d90345 (2001-02-03 08:05:11) squash commits into 6b1027d2770cd0a39c468e525e52bf8c47e1464a

    Changed commits:
    ○  + mzvwutvl 9f4fb57f (empty) (no description set)
    │ ○  - rlvkpnrz hidden 1d7f8f94 (no description set)
    ├─╯  diff --git a/file b/file
    │    index 7898192261..6178079822 100644
    │    --- a/file
    │    +++ b/file
    │    @@ -1,1 +1,1 @@
    │    -a
    │    +b
    ○  + qpvuntsm 2ac85fd1 (no description set)
       - qpvuntsm hidden 6b1027d2 (no description set)
       diff --git a/file b/file
       index 7898192261..6178079822 100644
       --- a/file
       +++ b/file
       @@ -1,1 +1,1 @@
       -a
       +b
    [EOF]
    ");

    // Abandon the working copy commit.
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["abandon"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Abandoned commit mzvwutvl 9f4fb57f (empty) (no description set)
    Working copy now at: yqosqzyt 33f321c4 (empty) (no description set)
    Parent commit      : qpvuntsm 2ac85fd1 (no description set)
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "diff", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    From operation: 894c12d90345 (2001-02-03 08:05:11) squash commits into 6b1027d2770cd0a39c468e525e52bf8c47e1464a
      To operation: e5505aa79d31 (2001-02-03 08:05:13) abandon commit 9f4fb57fba25a7b47ce5980a5d9a4766778331e8

    Changed commits:
    ○  + yqosqzyt 33f321c4 (empty) (no description set)
    ○  - mzvwutvl hidden 9f4fb57f (empty) (no description set)
    [EOF]
    ");
}

#[test]
fn test_op_diff_sibling() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "log", "--no-graph", r#"-Tid.short() ++ "\n""#],
    );
    let base_op_id = stdout.raw().lines().next().unwrap();
    insta::assert_snapshot!(base_op_id, @"eac759b9ab75");

    // Create merge commit at one operation side. The parent trees will have to
    // be merged when diffing, which requires the commit index of this side.
    test_env.jj_cmd_ok(&repo_path, &["new", "root()", "-mA.1"]);
    std::fs::write(repo_path.join("file1"), "a\n").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["new", "root()", "-mA.2"]);
    std::fs::write(repo_path.join("file2"), "a\n").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["new", "all:@-+", "-mA"]);

    // Create another operation diverged from the base operation.
    test_env.jj_cmd_ok(&repo_path, &["describe", "--at-op", base_op_id, "-mB"]);

    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(&stdout, @r"
    @    779ecb7ea7f0 test-username@host.example.com 2001-02-03 04:05:13.000 +07:00 - 2001-02-03 04:05:13.000 +07:00
    ├─╮  reconcile divergent operations
    │ │  args: jj op log
    ○ │  d700dc16fded test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    │ │  new empty commit
    │ │  args: jj new 'all:@-+' -mA
    ○ │  b47de32023e1 test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    │ │  snapshot working copy
    │ │  args: jj new 'all:@-+' -mA
    ○ │  8a31868f615d test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │ │  new empty commit
    │ │  args: jj new 'root()' -mA.2
    ○ │  2cd33ddecde8 test-username@host.example.com 2001-02-03 04:05:10.000 +07:00 - 2001-02-03 04:05:10.000 +07:00
    │ │  snapshot working copy
    │ │  args: jj new 'root()' -mA.2
    ○ │  d86c1ae55c48 test-username@host.example.com 2001-02-03 04:05:09.000 +07:00 - 2001-02-03 04:05:09.000 +07:00
    │ │  new empty commit
    │ │  args: jj new 'root()' -mA.1
    │ ○  13b143e1f4f9 test-username@host.example.com 2001-02-03 04:05:12.000 +07:00 - 2001-02-03 04:05:12.000 +07:00
    ├─╯  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │    args: jj describe --at-op eac759b9ab75 -mB
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");
    insta::assert_snapshot!(&stderr, @r"
    Concurrent modification detected, resolving automatically.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &["op", "log", "--no-graph", r#"-Tid.short() ++ "\n""#],
    );
    let (head_op_id, p1_op_id, _, _, _, _, p2_op_id) = stdout.raw().lines().next_tuple().unwrap();
    insta::assert_snapshot!(head_op_id, @"779ecb7ea7f0");
    insta::assert_snapshot!(p1_op_id, @"d700dc16fded");
    insta::assert_snapshot!(p2_op_id, @"13b143e1f4f9");

    // Diff between p1 and p2 operations should work no matter if p2 is chosen
    // as a base operation.
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &[
            "op",
            "diff",
            "--at-op",
            p1_op_id,
            "--from",
            p1_op_id,
            "--to",
            p2_op_id,
            "--summary",
        ],
    );
    insta::assert_snapshot!(&stdout, @r"
    From operation: d700dc16fded (2001-02-03 08:05:11) new empty commit
      To operation: 13b143e1f4f9 (2001-02-03 08:05:12) describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22

    Changed commits:
    ○  + qpvuntsm 02ef2bc4 (empty) B
    ○    - mzvwutvl hidden 270db3d9 (empty) A
    ├─╮
    │ ○  - kkmpptxz hidden 8331e0a3 A.1
    │    A file1
    ○  - zsuskuln hidden 8afecaef A.2
       A file2
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(
        &repo_path,
        &[
            "op",
            "diff",
            "--at-op",
            p2_op_id,
            "--from",
            p2_op_id,
            "--to",
            p1_op_id,
            "--summary",
        ],
    );
    insta::assert_snapshot!(&stdout, @r"
    From operation: 13b143e1f4f9 (2001-02-03 08:05:12) describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
      To operation: d700dc16fded (2001-02-03 08:05:11) new empty commit

    Changed commits:
    ○    + mzvwutvl 270db3d9 (empty) A
    ├─╮
    │ ○  + kkmpptxz 8331e0a3 A.1
    │    A file1
    ○  + zsuskuln 8afecaef A.2
       A file2
    ○  - qpvuntsm hidden 02ef2bc4 (empty) B
    [EOF]
    ");
}

#[test]
fn test_op_diff_word_wrap() {
    let test_env = TestEnvironment::default();
    let git_repo_path = test_env.env_root().join("git-repo");
    init_bare_git_repo(&git_repo_path);
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "clone", "git-repo", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    let render = |args: &[&str], columns: u32, word_wrap: bool| {
        let mut args = args.to_vec();
        if word_wrap {
            args.push("--config=ui.log-word-wrap=true");
        }
        let assert = test_env
            .jj_cmd(&repo_path, &args)
            .env("COLUMNS", columns.to_string())
            .assert()
            .success()
            .stderr("");
        get_stdout_string(&assert)
    };

    // Add some file content changes
    std::fs::write(repo_path.join("file1"), "foo\n".repeat(100)).unwrap();
    test_env.jj_cmd_ok(&repo_path, &["debug", "snapshot"]);

    // ui.log-word-wrap option works, and diff stat respects content width
    insta::assert_snapshot!(render(&["op", "diff", "--from=@---", "--stat"], 40, true), @r"
    From operation: eac759b9ab75 (2001-02-03 08:05:07) add workspace 'default'
      To operation: e1a943dc0fe5 (2001-02-03 08:05:08) snapshot working copy

    Changed commits:
    ○  + sqpuoqvx 7581c520 (no description
    │  set)
    │  file1 | 100 ++++++++++++++++++++++
    │  1 file changed, 100 insertions(+), 0 deletions(-)
    ○  + pukowqtp 0cb7e07e bookmark-1 |
       Commit 1
       some-file | 1 +
       1 file changed, 1 insertion(+), 0 deletions(-)
    ○  + rnnslrkn 4ff62539 bookmark-2@origin
       | Commit 2
       some-file | 1 +
       1 file changed, 1 insertion(+), 0 deletions(-)
    ○  + rnnkyono 11671e4c bookmark-3@origin
       | Commit 3
       some-file | 1 +
       1 file changed, 1 insertion(+), 0 deletions(-)
    ○  - qpvuntsm hidden 230dd059 (empty)
       (no description set)
       0 files changed, 0 insertions(+), 0 deletions(-)

    Changed local bookmarks:
    bookmark-1:
    + pukowqtp 0cb7e07e bookmark-1 | Commit
    1
    - (absent)

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked pukowqtp 0cb7e07e bookmark-1 |
    Commit 1
    - untracked (absent)
    bookmark-2@origin:
    + untracked rnnslrkn 4ff62539
    bookmark-2@origin | Commit 2
    - untracked (absent)
    bookmark-3@origin:
    + untracked rnnkyono 11671e4c
    bookmark-3@origin | Commit 3
    - untracked (absent)
    ");

    // Graph width should be subtracted from the term width
    let config = r#"templates.commit_summary='"0 1 2 3 4 5 6 7 8 9"'"#;
    insta::assert_snapshot!(
        render(&["op", "diff", "--from=@---", "--config", config], 10, true), @r"
    From operation: eac759b9ab75 (2001-02-03 08:05:07) add workspace 'default'
      To operation: e1a943dc0fe5 (2001-02-03 08:05:08) snapshot working copy

    Changed
    commits:
    ○  + 0 1 2
    │  3 4 5 6
    │  7 8 9
    ○  + 0 1 2
       3 4 5 6
       7 8 9
    ○  + 0 1 2
       3 4 5 6
       7 8 9
    ○  + 0 1 2
       3 4 5 6
       7 8 9
    ○  - 0 1 2
       3 4 5 6
       7 8 9

    Changed
    local
    bookmarks:
    bookmark-1:
    + 0 1 2 3
    4 5 6 7 8
    9
    - (absent)

    Changed
    remote
    bookmarks:
    bookmark-1@origin:
    + tracked
    0 1 2 3 4
    5 6 7 8 9
    -
    untracked
    (absent)
    bookmark-2@origin:
    +
    untracked
    0 1 2 3 4
    5 6 7 8 9
    -
    untracked
    (absent)
    bookmark-3@origin:
    +
    untracked
    0 1 2 3 4
    5 6 7 8 9
    -
    untracked
    (absent)
    ");
}

#[test]
fn test_op_show() {
    let test_env = TestEnvironment::default();
    let git_repo_path = test_env.env_root().join("git-repo");
    let git_repo = init_bare_git_repo(&git_repo_path);
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "clone", "git-repo", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    // Overview of op log.
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(&stdout, @r"
    @  364d0a677b0c test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  check out git remote's default branch
    │  args: jj git clone git-repo repo
    ○  369ee2939177 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  fetch from git remote into empty repo
    │  args: jj git clone git-repo repo
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  000000000000 root()
    [EOF]
    ");

    // The root operation is empty.
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show", "0000000"]);
    insta::assert_snapshot!(&stdout, @r"
    000000000000 root()
    [EOF]
    ");

    // Showing the latest operation.
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show", "@"]);
    insta::assert_snapshot!(&stdout, @r"
    364d0a677b0c test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    check out git remote's default branch
    args: jj git clone git-repo repo

    Changed commits:
    ○  + sqpuoqvx c7b48fea (empty) (no description set)
    ○  - qpvuntsm hidden 230dd059 (empty) (no description set)

    Changed local bookmarks:
    bookmark-1:
    + pukowqtp 0cb7e07e bookmark-1 | Commit 1
    - (absent)

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked pukowqtp 0cb7e07e bookmark-1 | Commit 1
    - untracked pukowqtp 0cb7e07e bookmark-1 | Commit 1
    [EOF]
    ");
    // `jj op show @` should behave identically to `jj op show`.
    let stdout_without_op_id = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    assert_eq!(stdout.raw(), stdout_without_op_id.raw());

    // Showing a given operation.
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show", "@-"]);
    insta::assert_snapshot!(&stdout, @r"
    369ee2939177 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    fetch from git remote into empty repo
    args: jj git clone git-repo repo

    Changed commits:
    ○  + rnnslrkn 4ff62539 bookmark-2@origin | Commit 2
    ○  + rnnkyono 11671e4c bookmark-3@origin | Commit 3
    ○  + pukowqtp 0cb7e07e bookmark-1@origin | Commit 1

    Changed remote bookmarks:
    bookmark-1@origin:
    + untracked pukowqtp 0cb7e07e bookmark-1@origin | Commit 1
    - untracked (absent)
    bookmark-2@origin:
    + untracked rnnslrkn 4ff62539 bookmark-2@origin | Commit 2
    - untracked (absent)
    bookmark-3@origin:
    + untracked rnnkyono 11671e4c bookmark-3@origin | Commit 3
    - untracked (absent)
    [EOF]
    ");

    // Create a conflicted bookmark using a concurrent operation.
    test_env.jj_cmd_ok(
        &repo_path,
        &[
            "bookmark",
            "set",
            "bookmark-1",
            "-r",
            "bookmark-2@origin",
            "--at-op",
            "@-",
        ],
    );
    let (_, stderr) = test_env.jj_cmd_ok(&repo_path, &["log"]);
    insta::assert_snapshot!(&stderr, @r"
    Concurrent modification detected, resolving automatically.
    [EOF]
    ");
    // Showing a merge operation is empty.
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    774687cc6e4e test-username@host.example.com 2001-02-03 04:05:14.000 +07:00 - 2001-02-03 04:05:14.000 +07:00
    reconcile divergent operations
    args: jj log
    [EOF]
    ");

    // Test fetching from git remote.
    modify_git_repo(git_repo);
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["git", "fetch"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    bookmark: bookmark-1@origin [updated] tracked
    bookmark: bookmark-2@origin [updated] untracked
    bookmark: bookmark-3@origin [deleted] untracked
    Abandoned 1 commits that are no longer reachable.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    fed3d4f59819 test-username@host.example.com 2001-02-03 04:05:16.000 +07:00 - 2001-02-03 04:05:16.000 +07:00
    fetch from git remote(s) origin
    args: jj git fetch

    Changed commits:
    ○  + kulxwnxm e1a239a5 bookmark-2@origin | Commit 5
    ○  + zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    ○  - rnnkyono hidden 11671e4c Commit 3

    Changed local bookmarks:
    bookmark-1:
    + (added) zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    + (added) rnnslrkn 4ff62539 bookmark-1?? | Commit 2
    - (added) pukowqtp 0cb7e07e Commit 1
    - (added) rnnslrkn 4ff62539 bookmark-1?? | Commit 2

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    - tracked pukowqtp 0cb7e07e Commit 1
    bookmark-2@origin:
    + untracked kulxwnxm e1a239a5 bookmark-2@origin | Commit 5
    - untracked rnnslrkn 4ff62539 bookmark-1?? | Commit 2
    bookmark-3@origin:
    + untracked (absent)
    - untracked rnnkyono hidden 11671e4c Commit 3
    [EOF]
    ");

    // Test creation of bookmark.
    let (stdout, stderr) = test_env.jj_cmd_ok(
        &repo_path,
        &[
            "bookmark",
            "create",
            "bookmark-2",
            "-r",
            "bookmark-2@origin",
        ],
    );
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Created 1 bookmarks pointing to kulxwnxm e1a239a5 bookmark-2 bookmark-2@origin | Commit 5
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    77ed6ae71f3f test-username@host.example.com 2001-02-03 04:05:18.000 +07:00 - 2001-02-03 04:05:18.000 +07:00
    create bookmark bookmark-2 pointing to commit e1a239a57eb15cefc5910198befbbbe2b43c47af
    args: jj bookmark create bookmark-2 -r bookmark-2@origin

    Changed local bookmarks:
    bookmark-2:
    + kulxwnxm e1a239a5 bookmark-2 bookmark-2@origin | Commit 5
    - (absent)
    [EOF]
    ");

    // Test tracking of a bookmark.
    let (stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["bookmark", "track", "bookmark-2@origin"]);
    insta::assert_snapshot!(&stdout, @r###"
     "###);
    insta::assert_snapshot!(&stderr, @r"
    Started tracking 1 remote bookmarks.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    f4e770fd7370 test-username@host.example.com 2001-02-03 04:05:20.000 +07:00 - 2001-02-03 04:05:20.000 +07:00
    track remote bookmark bookmark-2@origin
    args: jj bookmark track bookmark-2@origin

    Changed remote bookmarks:
    bookmark-2@origin:
    + tracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    - untracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    [EOF]
    ");

    // Test creation of new commit.
    let (stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["bookmark", "track", "bookmark-2@origin"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Warning: Remote bookmark already tracked: bookmark-2@origin
    Nothing changed.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    f4e770fd7370 test-username@host.example.com 2001-02-03 04:05:20.000 +07:00 - 2001-02-03 04:05:20.000 +07:00
    track remote bookmark bookmark-2@origin
    args: jj bookmark track bookmark-2@origin

    Changed remote bookmarks:
    bookmark-2@origin:
    + tracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    - untracked kulxwnxm e1a239a5 bookmark-2 | Commit 5
    [EOF]
    ");

    // Test creation of new commit.
    let (stdout, stderr) = test_env.jj_cmd_ok(
        &repo_path,
        &["new", "bookmark-1@origin", "-m", "new commit"],
    );
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Working copy now at: xznxytkn 560df364 (empty) new commit
    Parent commit      : zkmtkqvo 0dee6313 bookmark-1?? bookmark-1@origin | Commit 4
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    e54bb5012457 test-username@host.example.com 2001-02-03 04:05:24.000 +07:00 - 2001-02-03 04:05:24.000 +07:00
    new empty commit
    args: jj new bookmark-1@origin -m 'new commit'

    Changed commits:
    ○  + xznxytkn 560df364 (empty) new commit
    ○  - sqpuoqvx hidden c7b48fea (empty) (no description set)
    [EOF]
    ");

    // Test updating of local bookmark.
    let (stdout, stderr) =
        test_env.jj_cmd_ok(&repo_path, &["bookmark", "set", "bookmark-1", "-r", "@"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Moved 1 bookmarks to xznxytkn 560df364 bookmark-1* | (empty) new commit
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    55527022d8b9 test-username@host.example.com 2001-02-03 04:05:26.000 +07:00 - 2001-02-03 04:05:26.000 +07:00
    point bookmark bookmark-1 to commit 560df364f0a09fe29f6a4fca8bd07c4464c7feee
    args: jj bookmark set bookmark-1 -r @

    Changed local bookmarks:
    bookmark-1:
    + xznxytkn 560df364 bookmark-1* | (empty) new commit
    - (added) zkmtkqvo 0dee6313 bookmark-1@origin | Commit 4
    - (added) rnnslrkn 4ff62539 Commit 2
    [EOF]
    ");

    // Test deletion of local bookmark.
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["bookmark", "delete", "bookmark-2"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Deleted 1 bookmarks.
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    a6838035030b test-username@host.example.com 2001-02-03 04:05:28.000 +07:00 - 2001-02-03 04:05:28.000 +07:00
    delete bookmark bookmark-2
    args: jj bookmark delete bookmark-2

    Changed local bookmarks:
    bookmark-2:
    + (absent)
    - kulxwnxm e1a239a5 bookmark-2@origin | Commit 5
    [EOF]
    ");

    // Test pushing to Git remote.
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["git", "push", "--tracked"]);
    insta::assert_snapshot!(&stdout, @r###"
    "###);
    insta::assert_snapshot!(&stderr, @r"
    Changes to push to origin:
      Move forward bookmark bookmark-1 from 0dee631320b1 to 560df364f0a0
      Delete bookmark bookmark-2 from e1a239a57eb1
    Warning: The working-copy commit in workspace 'default' became immutable, so a new commit has been created on top of it.
    Working copy now at: pzsxstzt 91310b51 (empty) (no description set)
    Parent commit      : xznxytkn 560df364 bookmark-1 | (empty) new commit
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show"]);
    insta::assert_snapshot!(&stdout, @r"
    2a7821341a99 test-username@host.example.com 2001-02-03 04:05:30.000 +07:00 - 2001-02-03 04:05:30.000 +07:00
    push all tracked bookmarks to git remote origin
    args: jj git push --tracked

    Changed commits:
    ○  + pzsxstzt 91310b51 (empty) (no description set)

    Changed remote bookmarks:
    bookmark-1@origin:
    + tracked xznxytkn 560df364 bookmark-1 | (empty) new commit
    - tracked zkmtkqvo 0dee6313 Commit 4
    bookmark-2@origin:
    + untracked (absent)
    - tracked kulxwnxm e1a239a5 Commit 5
    [EOF]
    ");
}

#[test]
fn test_op_show_patch() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    // Update working copy with a single file and create new commit.
    std::fs::write(repo_path.join("file"), "a\n").unwrap();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["new"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Working copy now at: rlvkpnrz 56950632 (empty) (no description set)
    Parent commit      : qpvuntsm 6b1027d2 (no description set)
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show", "@-", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    187a5a9d8a22 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    snapshot working copy
    args: jj new

    Changed commits:
    ○  + qpvuntsm 6b1027d2 (no description set)
       - qpvuntsm hidden 230dd059 (empty) (no description set)
       diff --git a/file b/file
       new file mode 100644
       index 0000000000..7898192261
       --- /dev/null
       +++ b/file
       @@ -0,0 +1,1 @@
       +a
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show", "@", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    a7e535e73c4b test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    new empty commit
    args: jj new

    Changed commits:
    ○  + rlvkpnrz 56950632 (empty) (no description set)
    [EOF]
    ");

    // Squash the working copy commit.
    std::fs::write(repo_path.join("file"), "b\n").unwrap();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["squash"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Working copy now at: mzvwutvl 9f4fb57f (empty) (no description set)
    Parent commit      : qpvuntsm 2ac85fd1 (no description set)
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    894c12d90345 test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    squash commits into 6b1027d2770cd0a39c468e525e52bf8c47e1464a
    args: jj squash

    Changed commits:
    ○  + mzvwutvl 9f4fb57f (empty) (no description set)
    │ ○  - rlvkpnrz hidden 1d7f8f94 (no description set)
    ├─╯  diff --git a/file b/file
    │    index 7898192261..6178079822 100644
    │    --- a/file
    │    +++ b/file
    │    @@ -1,1 +1,1 @@
    │    -a
    │    +b
    ○  + qpvuntsm 2ac85fd1 (no description set)
       - qpvuntsm hidden 6b1027d2 (no description set)
       diff --git a/file b/file
       index 7898192261..6178079822 100644
       --- a/file
       +++ b/file
       @@ -1,1 +1,1 @@
       -a
       +b
    [EOF]
    ");

    // Abandon the working copy commit.
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["abandon"]);
    insta::assert_snapshot!(&stdout, @"");
    insta::assert_snapshot!(&stderr, @r"
    Abandoned commit mzvwutvl 9f4fb57f (empty) (no description set)
    Working copy now at: yqosqzyt 33f321c4 (empty) (no description set)
    Parent commit      : qpvuntsm 2ac85fd1 (no description set)
    [EOF]
    ");
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "show", "-p", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    e5505aa79d31 test-username@host.example.com 2001-02-03 04:05:13.000 +07:00 - 2001-02-03 04:05:13.000 +07:00
    abandon commit 9f4fb57fba25a7b47ce5980a5d9a4766778331e8
    args: jj abandon

    Changed commits:
    ○  + yqosqzyt 33f321c4 (empty) (no description set)
    ○  - mzvwutvl hidden 9f4fb57f (empty) (no description set)
    [EOF]
    ");

    // Try again with "op log".
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "--git"]);
    insta::assert_snapshot!(&stdout, @r"
    @  e5505aa79d31 test-username@host.example.com 2001-02-03 04:05:13.000 +07:00 - 2001-02-03 04:05:13.000 +07:00
    │  abandon commit 9f4fb57fba25a7b47ce5980a5d9a4766778331e8
    │  args: jj abandon
    │
    │  Changed commits:
    │  ○  + yqosqzyt 33f321c4 (empty) (no description set)
    │  ○  - mzvwutvl hidden 9f4fb57f (empty) (no description set)
    ○  894c12d90345 test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    │  squash commits into 6b1027d2770cd0a39c468e525e52bf8c47e1464a
    │  args: jj squash
    │
    │  Changed commits:
    │  ○  + mzvwutvl 9f4fb57f (empty) (no description set)
    │  │ ○  - rlvkpnrz hidden 1d7f8f94 (no description set)
    │  ├─╯  diff --git a/file b/file
    │  │    index 7898192261..6178079822 100644
    │  │    --- a/file
    │  │    +++ b/file
    │  │    @@ -1,1 +1,1 @@
    │  │    -a
    │  │    +b
    │  ○  + qpvuntsm 2ac85fd1 (no description set)
    │     - qpvuntsm hidden 6b1027d2 (no description set)
    │     diff --git a/file b/file
    │     index 7898192261..6178079822 100644
    │     --- a/file
    │     +++ b/file
    │     @@ -1,1 +1,1 @@
    │     -a
    │     +b
    ○  15c3c5d0baf0 test-username@host.example.com 2001-02-03 04:05:11.000 +07:00 - 2001-02-03 04:05:11.000 +07:00
    │  snapshot working copy
    │  args: jj squash
    │
    │  Changed commits:
    │  ○  + rlvkpnrz 1d7f8f94 (no description set)
    │     - rlvkpnrz hidden 56950632 (empty) (no description set)
    │     diff --git a/file b/file
    │     index 7898192261..6178079822 100644
    │     --- a/file
    │     +++ b/file
    │     @@ -1,1 +1,1 @@
    │     -a
    │     +b
    ○  a7e535e73c4b test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  new empty commit
    │  args: jj new
    │
    │  Changed commits:
    │  ○  + rlvkpnrz 56950632 (empty) (no description set)
    ○  187a5a9d8a22 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  snapshot working copy
    │  args: jj new
    │
    │  Changed commits:
    │  ○  + qpvuntsm 6b1027d2 (no description set)
    │     - qpvuntsm hidden 230dd059 (empty) (no description set)
    │     diff --git a/file b/file
    │     new file mode 100644
    │     index 0000000000..7898192261
    │     --- /dev/null
    │     +++ b/file
    │     @@ -0,0 +1,1 @@
    │     +a
    ○  eac759b9ab75 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    │
    │  Changed commits:
    │  ○  + qpvuntsm 230dd059 (empty) (no description set)
    ○  000000000000 root()
    [EOF]
    ");
}

fn init_bare_git_repo(git_repo_path: &Path) -> gix::Repository {
    let git_repo = git::init_bare(git_repo_path);
    let commit_result = git::add_commit(
        &git_repo,
        "refs/heads/bookmark-1",
        "some-file",
        b"some content",
        "Commit 1",
        &[],
    );
    git::write_commit(
        &git_repo,
        "refs/heads/bookmark-2",
        commit_result.tree_id,
        "Commit 2",
        &[],
    );
    git::write_commit(
        &git_repo,
        "refs/heads/bookmark-3",
        commit_result.tree_id,
        "Commit 3",
        &[],
    );

    git::set_head_to_id(&git_repo, commit_result.commit_id);
    git_repo
}

fn modify_git_repo(git_repo: gix::Repository) -> gix::Repository {
    let bookmark1_commit = git_repo
        .find_reference("refs/heads/bookmark-1")
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id();
    let bookmark2_commit = git_repo
        .find_reference("refs/heads/bookmark-2")
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id();

    let commit_result = git::add_commit(
        &git_repo,
        "refs/heads/bookmark-1",
        "next-file",
        b"more content",
        "Commit 4",
        &[bookmark1_commit.detach()],
    );
    git::write_commit(
        &git_repo,
        "refs/heads/bookmark-2",
        commit_result.tree_id,
        "Commit 5",
        &[bookmark2_commit.detach()],
    );

    git_repo
        .find_reference("refs/heads/bookmark-3")
        .unwrap()
        .delete()
        .unwrap();
    git_repo
}

#[must_use]
fn get_log_output(test_env: &TestEnvironment, repo_path: &Path, op_id: &str) -> CommandOutput {
    test_env.run_jj_in(
        repo_path,
        ["log", "-T", "commit_id", "--at-op", op_id, "-r", "all()"],
    )
}
