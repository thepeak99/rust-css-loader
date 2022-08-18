//#![feature(proc_macro_span)]

use cssparser::{ParseError, Parser, ParserInput, ToCss, Token};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use syn::LitStr;

fn get_base_name(filename: &Path, full_path: bool) -> Result<String, Box<dyn Error>> {
    let out = if full_path {
        filename
            .parent()
            .ok_or("Can't get parent name")?
            .join(filename.file_stem().ok_or("Can't get file stem")?)
            .to_str()
            .ok_or("Can.t convert path to string")?
            .to_owned()
            .replace("/", "_")
    } else {
        filename
            .file_stem()
            .ok_or("Can't get file stem")?
            .to_str()
            .ok_or("Can't get file stem")?
            .to_owned()
    };

    Ok(out)
}

fn parse_css(
    filename: &Path,
    css_source: &[u8],
) -> Result<(Vec<(String, String)>, String), Box<dyn Error>> {
    let css_source = String::from_utf8(css_source.into())?;
    let mut input = ParserInput::new(&css_source);
    let mut parser = Parser::new(&mut input);

    let mut css_out = String::new();
    let mut idents = Vec::new();

    let mut is_class = true;
    while !parser.is_exhausted() {
        match parser.next().unwrap() {
            Token::Ident(i) => {
                if is_class {
                    css_out += i;
                } else {
                    let tag = get_base_name(filename, true)?;
                    let tagged_ident = format!("{}-{}", tag, i.to_string());

                    css_out += &format!(".{}", tagged_ident);
                    idents.push((i.to_string(), tagged_ident));
                }
            }
            Token::CurlyBracketBlock => {
                parser
                    .parse_nested_block(|parser| -> Result<(), ParseError<'_, &String>> {
                        css_out += "{";
                        while !parser.is_exhausted() {
                            css_out += &parser.next().unwrap().to_css_string();
                        }
                        css_out += "}";
                        Ok(())
                    })
                    .map_err(|_| "Can't parse CSS")?;
                is_class = true;
            }
            Token::Delim('.') => {
                is_class = false;
            }
            token => {
                println!("{:?}", token);
                css_out += &token.to_css_string();
            }
        }
    }

    Ok((idents, css_out))
}

fn compile_sass(source: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    rsass::compile_scss(
        source,
        rsass::output::Format {
            style: rsass::output::Style::Compressed,
            ..Default::default()
        },
    )
    .map_err(|e| format!("Can't parse SASS {}", e.to_string()).into())
}

fn load_css(filename: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let is_sass = if filename.extension().ok_or("Can't get extensios")? == "scss" {
        true
    } else {
        false
    };

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let full_path = manifest_dir.join(filename);

    let source = fs::read(full_path)?;

    if is_sass {
        compile_sass(&source)
    } else {
        Ok(source)
    }
}

/// Reads the specified CSS or SASS file into the project, and then it scopes it to the local file.
/// Paths are relative to CARGO_MANIFEST_DIR.
#[proc_macro]
pub fn import_style(item: TokenStream) -> TokenStream {
    let filename: PathBuf = syn::parse::<LitStr>(item).unwrap().value().into();

    let css = load_css(&filename).unwrap();
    let (idents, css) = parse_css(&filename, &css).unwrap();

    let base_name = get_base_name(&filename, false).unwrap();
    let inner_struct_name = format_ident!("__{}", base_name);

    let inner_struct_fields = idents.iter().map(|(ident, _)| {
        let ident = Ident::new(ident, Span::call_site());
        quote! { #ident: &'static str, }
    });

    let outer_struct_name = format_ident!("_{}", base_name);

    let inner_struct_values = idents.iter().map(|(ident, new_ident)| {
        let ident = format_ident!("{}", ident);
        quote! { #ident: #new_ident, }
    });

    let base_name_ident = format_ident!("{}", base_name);

    quote! {
        use std::ops::Deref;

        struct #inner_struct_name {
            #(#inner_struct_fields)*
        }

        struct #outer_struct_name {
            css: &'static str,
            names: #inner_struct_name,
            initialized: bool,
        }

        static #base_name_ident: #outer_struct_name = #outer_struct_name {
            css: #css,
            names: #inner_struct_name {
                #(#inner_struct_values)*
            },
            initialized: false,
        };

        impl Deref for #outer_struct_name {
            type Target = #inner_struct_name;

            fn deref(&self) -> &Self::Target {
                if !self.initialized {
                    let init = &self.initialized as *const bool;
                    let init_mut = init as *mut bool;
                    unsafe {
                        *init_mut = true;
                    }

                    let window = css_loader::web_sys::window().unwrap();
                    let document = window.document().unwrap();
                    let css = document.create_element("style").unwrap();
                    css.set_text_content(Some(self.css));
                    document.head().unwrap().append_child(&css).unwrap();
                }

                &self.names
            }
        }
    }
    .into()
}
