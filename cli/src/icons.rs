use std::io::IsTerminal;

const DIRECTORY: &str = "";
const FILE: &str = "";

const DIRECTORY_ICONS: &[(&str, &str)] = &[
    (".git", ""),
    (".github", ""),
    (".vscode", ""),
    (".idea", ""),
    ("node_modules", ""),
    ("src", ""),
    ("tests", "󰙨"),
    ("target", ""),
    ("build", "󰉋"),
    ("dist", "󰉋"),
    ("out", "󰉋"),
    ("bin", ""),
    ("lib", ""),
    ("libs", ""),
    ("vendor", ""),
    ("assets", "󰉏"),
    ("images", "󰉏"),
    ("img", "󰉏"),
    ("pictures", ""),
    ("photos", ""),
    ("music", ""),
    ("videos", ""),
    ("video", ""),
    ("downloads", ""),
    ("docs", "󰈙"),
    ("doc", "󰈙"),
    ("documentation", "󰈙"),
    ("fonts", ""),
    ("public", ""),
    ("database", ""),
    ("db", ""),
    ("config", ""),
    ("configs", ""),
];

const NAME_ICONS: &[(&str, &str)] = &[
    ("dockerfile", ""),
    ("containerfile", ""),
    ("makefile", ""),
    ("gnumakefile", ""),
    ("justfile", ""),
    ("cargo.toml", ""),
    ("cargo.lock", ""),
    ("go.mod", ""),
    ("go.sum", ""),
    ("package.json", ""),
    ("package-lock.json", ""),
    ("pnpm-lock.yaml", ""),
    ("yarn.lock", ""),
    ("composer.json", ""),
    ("composer.lock", ""),
    ("gemfile", ""),
    ("gemfile.lock", ""),
    ("readme", ""),
    ("license", ""),
    ("license.md", ""),
    (".gitignore", ""),
    (".gitattributes", ""),
    (".gitmodules", ""),
    (".env", ""),
    (".env.local", ""),
    (".env.example", ""),
];

const FILE_ICONS: &[(&str, &str)] = &[
    // Systems and programming languages
    ("rs", ""),
    ("c", ""),
    ("h", ""),
    ("cc", ""),
    ("cpp", ""),
    ("cxx", ""),
    ("hh", ""),
    ("hpp", ""),
    ("hxx", ""),
    ("go", ""),
    ("py", ""),
    ("pyi", ""),
    ("pyw", ""),
    ("pyc", ""),
    ("js", ""),
    ("mjs", ""),
    ("cjs", ""),
    ("jsx", ""),
    ("ts", ""),
    ("mts", ""),
    ("cts", ""),
    ("tsx", ""),
    ("java", ""),
    ("class", ""),
    ("jar", ""),
    ("kt", ""),
    ("kts", ""),
    ("swift", ""),
    ("dart", ""),
    ("lua", ""),
    ("rb", ""),
    ("rake", ""),
    ("gemspec", ""),
    ("php", ""),
    ("phar", ""),
    ("hs", ""),
    ("lhs", ""),
    ("scala", ""),
    ("sc", ""),
    ("fs", ""),
    ("fsx", ""),
    ("fsi", ""),
    ("cs", ""),
    ("csx", ""),
    ("vb", ""),
    ("pl", ""),
    ("pm", ""),
    ("r", "󰟔"),
    ("jl", ""),
    ("ex", ""),
    ("exs", ""),
    ("erl", ""),
    ("hrl", ""),
    ("clj", ""),
    ("cljs", ""),
    ("cljc", ""),
    ("edn", ""),
    ("elm", ""),
    ("vue", "󰡄"),
    ("svelte", ""),
    ("astro", ""),
    ("sol", ""),
    ("vy", ""),
    ("zig", ""),
    ("nim", ""),
    ("cr", ""),
    ("v", ""),
    ("sv", ""),
    ("svh", ""),
    ("vhd", ""),
    ("vhdl", ""),
    ("asm", ""),
    ("s", ""),
    ("S", ""),
    ("nasm", ""),
    ("sh", ""),
    ("bash", ""),
    ("zsh", ""),
    ("fish", ""),
    ("ksh", ""),
    ("ps1", "󰨊"),
    ("psm1", "󰨊"),
    ("bat", "󰆍"),
    ("cmd", "󰆍"),
    ("sql", ""),
    ("graphql", ""),
    ("gql", ""),
    ("proto", "󰗀"),
    ("wasm", ""),
    ("wat", ""),
    ("ipynb", ""),
    // Web, markup and data
    ("html", ""),
    ("htm", ""),
    ("xhtml", ""),
    ("css", ""),
    ("scss", ""),
    ("sass", ""),
    ("less", ""),
    ("styl", ""),
    ("json", ""),
    ("jsonc", ""),
    ("json5", ""),
    ("toml", ""),
    ("yaml", ""),
    ("yml", ""),
    ("xml", "󰗀"),
    ("xsd", "󰗀"),
    ("xsl", "󰗀"),
    ("xslt", "󰗀"),
    ("ini", ""),
    ("cfg", ""),
    ("conf", ""),
    ("config", ""),
    ("properties", ""),
    ("env", ""),
    ("md", ""),
    ("markdown", ""),
    ("mdx", ""),
    ("rst", ""),
    ("adoc", ""),
    ("txt", ""),
    ("text", ""),
    ("log", "󰌱"),
    ("csv", ""),
    ("tsv", ""),
    ("tex", ""),
    ("bib", ""),
    // Images and design assets
    ("png", ""),
    ("jpg", ""),
    ("jpeg", ""),
    ("gif", ""),
    ("webp", ""),
    ("bmp", ""),
    ("ico", ""),
    ("icns", ""),
    ("tif", ""),
    ("tiff", ""),
    ("svg", "󰜡"),
    ("avif", ""),
    ("heic", ""),
    ("heif", ""),
    ("raw", ""),
    ("dng", ""),
    ("psd", ""),
    ("ai", ""),
    ("eps", ""),
    ("sketch", ""),
    ("fig", ""),
    ("figma", ""),
    ("blend", "󰆧"),
    ("obj", "󰆧"),
    ("fbx", "󰆧"),
    ("stl", "󰆧"),
    ("gltf", "󰆧"),
    ("glb", "󰆧"),
    // Audio and video
    ("mp3", ""),
    ("wav", ""),
    ("flac", ""),
    ("aac", ""),
    ("m4a", ""),
    ("ogg", ""),
    ("opus", ""),
    ("mid", ""),
    ("midi", ""),
    ("wma", ""),
    ("aiff", ""),
    ("aif", ""),
    ("mp4", ""),
    ("mkv", ""),
    ("mov", ""),
    ("avi", ""),
    ("webm", ""),
    ("flv", ""),
    ("m4v", ""),
    ("wmv", ""),
    ("mpeg", ""),
    ("mpg", ""),
    ("3gp", ""),
    // Documents and office files
    ("pdf", ""),
    ("doc", ""),
    ("docx", ""),
    ("odt", ""),
    ("rtf", ""),
    ("xls", ""),
    ("xlsx", ""),
    ("ods", ""),
    ("ppt", ""),
    ("pptx", ""),
    ("odp", ""),
    ("epub", ""),
    ("mobi", ""),
    ("azw", ""),
    ("azw3", ""),
    // Archives, packages and disk images
    ("zip", ""),
    ("tar", ""),
    ("gz", ""),
    ("bz2", ""),
    ("xz", ""),
    ("zst", ""),
    ("zstd", ""),
    ("7z", ""),
    ("rar", ""),
    ("tgz", ""),
    ("tbz", ""),
    ("tbz2", ""),
    ("txz", ""),
    ("tlz", ""),
    ("br", ""),
    ("deb", ""),
    ("rpm", ""),
    ("apk", ""),
    ("ipa", ""),
    ("iso", ""),
    ("dmg", ""),
    ("img", ""),
    ("msi", ""),
    ("pkg", ""),
    ("whl", ""),
    ("nupkg", ""),
    ("war", ""),
    // Databases, binaries, certificates and fonts
    ("db", ""),
    ("sqlite", ""),
    ("sqlite3", ""),
    ("sqlitedb", ""),
    ("mdb", ""),
    ("accdb", ""),
    ("dump", ""),
    ("bak", "󰁯"),
    ("exe", ""),
    ("dll", ""),
    ("so", ""),
    ("dylib", ""),
    ("bin", ""),
    ("elf", ""),
    ("o", ""),
    ("obj", ""),
    ("a", ""),
    ("lib", ""),
    ("pem", "󰌾"),
    ("crt", "󰌾"),
    ("cer", "󰌾"),
    ("p12", "󰌾"),
    ("pfx", "󰌾"),
    ("key", "󰌾"),
    ("pub", "󰌾"),
    ("der", "󰌾"),
    ("ttf", ""),
    ("otf", ""),
    ("woff", ""),
    ("woff2", ""),
    ("eot", ""),
];

pub fn enabled() -> bool {
    std::io::stdout().is_terminal()
}

pub fn for_entry(name: &str, is_dir: bool) -> &'static str {
    let name = name.trim_end_matches('/');

    if is_dir {
        return DIRECTORY_ICONS
            .iter()
            .find(|(candidate, _)| name.eq_ignore_ascii_case(candidate))
            .map(|(_, icon)| *icon)
            .unwrap_or(DIRECTORY);
    }

    if let Some((_, icon)) = NAME_ICONS
        .iter()
        .find(|(candidate, _)| name.eq_ignore_ascii_case(candidate))
    {
        return *icon;
    }

    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or("");

    FILE_ICONS
        .iter()
        .find(|(candidate, _)| extension.eq_ignore_ascii_case(candidate))
        .map(|(_, icon)| *icon)
        .unwrap_or(FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_common_file_types() {
        for name in [
            "main.rs",
            "engine.cpp",
            "script.py",
            "app.tsx",
            "config.toml",
            "data.yaml",
            "page.html",
            "style.scss",
            "photo.webp",
            "track.flac",
            "movie.mkv",
            "report.pdf",
            "archive.tar.gz",
            "database.sqlite3",
            "font.woff2",
        ] {
            assert_ne!(for_entry(name, false), FILE, "{name}");
        }
    }

    #[test]
    fn recognizes_special_names_and_directories() {
        assert_eq!(for_entry("Dockerfile", false), "");
        assert_eq!(for_entry(".gitignore", false), "");
        assert_eq!(for_entry("node_modules", true), "");
        assert_eq!(for_entry("anything", true), DIRECTORY);
    }

    #[test]
    fn unknown_files_use_a_generic_icon() {
        assert_eq!(for_entry("unknown.extension", false), FILE);
        assert_eq!(for_entry("README", false), "");
    }
}
