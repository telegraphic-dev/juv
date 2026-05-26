use doj::{parse_directives, split_directive_words};

#[test]
fn parses_core_jbang_directives() {
    let src = r#"
//JAVA 21
//DEPS org.slf4j:slf4j-simple:2.0.13, org.slf4j:slf4j-api:2.0.13
//REPOS central=https://repo1.maven.org/maven2
//SOURCES helper.java
//JAVAC_OPTIONS --enable-preview "-Xlint:all"
//RUNTIME_OPTIONS --enable-preview '-Ddemo=true'
//MAIN com.acme.Main
//PREVIEW
class Main {}
"#;

    let directives = parse_directives(src);
    assert_eq!(directives.java_version.as_deref(), Some("21"));
    assert_eq!(directives.main_class.as_deref(), Some("com.acme.Main"));
    assert_eq!(
        directives.deps,
        vec![
            "org.slf4j:slf4j-simple:2.0.13",
            "org.slf4j:slf4j-api:2.0.13"
        ]
    );
    assert_eq!(
        directives.repos,
        vec!["central=https://repo1.maven.org/maven2"]
    );
    assert_eq!(directives.sources, vec!["helper.java"]);
    assert!(directives.enable_preview);
    assert_eq!(
        directives.javac_options,
        vec!["--enable-preview", "-Xlint:all"]
    );
    assert_eq!(
        directives.runtime_options,
        vec!["--enable-preview", "-Ddemo=true"]
    );
}

#[test]
fn splits_like_jbang_spaces_semicolons_commas_tabs_with_quotes() {
    assert_eq!(
        split_directive_words(r#"a:b:1, c:d:2; "quoted value" 'single value'"#),
        vec!["a:b:1", "c:d:2", "quoted value", "single value"]
    );
}
