//! Codegen snapshot tests.
//!
//! Each fixture in `tests/fixtures/snapshots/*.ruitl` is parsed and run through
//! the code generator, then pretty-printed via `prettyplease` and snapshotted
//! with `insta`. A diff in generated output surfaces in a readable `.snap`
//! diff rather than a brittle `.contains(...)` failure.
//!
//! To regenerate after intentional codegen changes:
//!   INSTA_UPDATE=always cargo test --test codegen_snapshots

use ruitl::codegen::CodeGenerator;
use ruitl::parser::RuitlParser;
use std::fs;
use std::path::Path;

fn render_snapshot(fixture: &str) -> String {
    // Go directly through parser + codegen (not compile_file_sibling) so the
    // snapshot content is independent of the `// ruitl-hash:` cache header
    // and of rustfmt availability on the host.
    let path = Path::new("tests/fixtures/snapshots").join(format!("{}.ruitl", fixture));
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    let mut parser = RuitlParser::new(source);
    let ast = parser
        .parse()
        .unwrap_or_else(|e| panic!("parse {}: {}", fixture, e));
    let mut gen = CodeGenerator::new(ast);
    let tokens = gen
        .generate()
        .unwrap_or_else(|e| panic!("codegen {}: {}", fixture, e));
    let file: syn::File = syn::parse2(tokens)
        .unwrap_or_else(|e| panic!("syn parse {}: {}", fixture, e));
    prettyplease::unparse(&file)
}

macro_rules! snap {
    ($name:ident) => {
        #[test]
        fn $name() {
            let out = render_snapshot(stringify!($name));
            insta::assert_snapshot!(stringify!($name), out);
        }
    };
}

snap!(props_only);
snap!(conditionals);
snap!(loops);
snap!(match_arms);
snap!(composition);
snap!(generics);
snap!(children);
