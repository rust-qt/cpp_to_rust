//! Types and functions used for Rust code generation.

#![allow(dead_code)]

use crate::doc_formatter;
use crate::rust_info::RustDatabase;
use crate::rust_info::RustDatabaseItem;
use crate::rust_info::RustEnumValue;
use crate::rust_info::RustFFIFunction;
use crate::rust_info::RustFfiWrapperData;
use crate::rust_info::RustFunction;
use crate::rust_info::RustFunctionArgument;
use crate::rust_info::RustFunctionKind;
use crate::rust_info::RustFunctionScope;
use crate::rust_info::RustItemKind;
use crate::rust_info::RustModule;
use crate::rust_info::RustStruct;
use crate::rust_info::RustStructKind;
use crate::rust_info::RustTraitImpl;
use crate::rust_info::RustWrapperTypeKind;
use crate::rust_type::CompleteType;
use crate::rust_type::RustPath;
use crate::rust_type::RustPointerLikeTypeKind;
use crate::rust_type::RustToFfiTypeConversion;
use crate::rust_type::RustType;
use itertools::Itertools;
use ritual_common::errors::{bail, err_msg, unexpected, Result};
use ritual_common::utils::MapIfOk;
use std::io;
use std::io::Write;

/// Generates Rust code representing type `rust_type` inside crate `crate_name`.
/// Same as `RustCodeGenerator::rust_type_to_code`, but accessible by other modules.
pub fn rust_type_to_code(rust_type: &RustType, current_crate: &str) -> String {
    match *rust_type {
        RustType::EmptyTuple => "()".to_string(),
        RustType::PointerLike {
            ref kind,
            ref target,
            ref is_const,
        } => {
            let target_code = rust_type_to_code(target, current_crate);
            match *kind {
                RustPointerLikeTypeKind::Pointer => {
                    if *is_const {
                        format!("*const {}", target_code)
                    } else {
                        format!("*mut {}", target_code)
                    }
                }
                RustPointerLikeTypeKind::Reference { ref lifetime } => {
                    let lifetime_text = match *lifetime {
                        Some(ref lifetime) => format!("'{} ", lifetime),
                        None => String::new(),
                    };
                    if *is_const {
                        format!("&{}{}", lifetime_text, target_code)
                    } else {
                        format!("&{}mut {}", lifetime_text, target_code)
                    }
                }
            }
        }
        RustType::Common {
            ref path,
            ref generic_arguments,
            ..
        } => {
            let mut code = path.full_name(Some(current_crate));
            if let Some(ref args) = *generic_arguments {
                code = format!(
                    "{}<{}>",
                    code,
                    args.iter()
                        .map(|x| rust_type_to_code(x, current_crate))
                        .join(", ",)
                );
            }
            code
        }
        RustType::FunctionPointer {
            ref return_type,
            ref arguments,
        } => format!(
            "extern \"C\" fn({}){}",
            arguments
                .iter()
                .map(|arg| rust_type_to_code(arg, current_crate))
                .join(", "),
            match return_type.as_ref() {
                &RustType::EmptyTuple => String::new(),
                return_type => format!(" -> {}", rust_type_to_code(return_type, current_crate)),
            }
        ),
    }
}

struct Generator<W> {
    #[allow(dead_code)]
    crate_name: String,
    destination: W,
}

impl<W: Write> Write for Generator<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.destination.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.destination.flush()
    }
}

/// Generates documentation comments containing
/// markdown code `doc`.
fn format_doc(doc: &str) -> String {
    format_doc_extended(doc, false)
}

/// Generates documentation comments containing
/// markdown code `doc`.
fn format_doc_extended(doc: &str, is_outer: bool) -> String {
    if doc.is_empty() {
        String::new()
    } else {
        doc.split('\n')
            .map(|x| {
                let prefix = if is_outer { "//! " } else { "/// " };

                if x.starts_with("    ") {
                    format!("{}{}", prefix, x.replace("    ", "&#32;   "))
                } else {
                    format!("{}{}", prefix, x)
                }
            })
            .join("")
    }
}

impl<W: Write> Generator<W> {
    fn generate_item(&mut self, item: &RustDatabaseItem, database: &RustDatabase) -> Result<()> {
        match item.kind {
            RustItemKind::Module(ref module) => self.generate_module(module, database),
            RustItemKind::Struct(ref data) => self.generate_struct(data, database),
            RustItemKind::EnumValue(ref value) => self.generate_enum_value(value),
            RustItemKind::TraitImpl(ref value) => self.generate_trait_impl(value),
            RustItemKind::Function(ref value) => self.generate_rust_final_function(value),
        }
    }

    fn rust_type_to_code(&self, rust_type: &RustType) -> String {
        rust_type_to_code(rust_type, &self.crate_name)
    }

    fn generate_module(&mut self, module: &RustModule, database: &RustDatabase) -> Result<()> {
        writeln!(
            self,
            "{}",
            format_doc(&doc_formatter::module_doc(&module.doc))
        )?;
        writeln!(self, "pub mod {} {{", module.path.last())?;

        self.generate_children(&module.path, database)?;

        writeln!(self, "}}")?;
        Ok(())
    }

    fn generate_struct(&mut self, rust_struct: &RustStruct, database: &RustDatabase) -> Result<()> {
        writeln!(
            self,
            "{}",
            format_doc(&doc_formatter::struct_doc(rust_struct))
        )?;
        let visibility = if rust_struct.is_public { "pub " } else { "" };
        match rust_struct.kind {
            RustStructKind::WrapperType(ref wrapper) => match wrapper.kind {
                RustWrapperTypeKind::EnumWrapper => {
                    writeln!(self, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]")?;
                    writeln!(
                        self,
                        "{}struct {}(::std::os::raw::c_int);",
                        visibility,
                        rust_struct.path.last()
                    )?;
                    writeln!(self)?;
                }
                _ => unimplemented!(),
            },
            RustStructKind::QtSlotWrapper(ref slot_wrapper) => {
                let arg_texts: Vec<_> = slot_wrapper
                    .arguments
                    .iter()
                    .map(|t| self.rust_type_to_code(&t.rust_api_type))
                    .collect();
                // TODO: use `doc_formatter` to generate doc instead of this
                let cpp_args = slot_wrapper
                    .arguments
                    .iter()
                    .map(|t| t.original_cpp_type.to_cpp_pseudo_code())
                    .join(", ");
                let args = arg_texts.join(", ");
                let args_tuple = format!("{}{}", args, if arg_texts.len() == 1 { "," } else { "" });
                let connections_mod =
                    RustPath::from_parts(vec!["qt_core".to_string(), "connection".to_string()])
                        .full_name(Some(&self.crate_name));
                let object_type_name = RustPath::from_parts(vec![
                    "qt_core".to_string(),
                    "object".to_string(),
                    "Object".to_string(),
                ])
                .full_name(Some(&self.crate_name));
                let callback_args = slot_wrapper
                    .arguments
                    .iter()
                    .enumerate()
                    .map(|(num, t)| {
                        format!("arg{}: {}", num, self.rust_type_to_code(&t.rust_ffi_type))
                    })
                    .join(", ");
                let func_args = slot_wrapper
                    .arguments
                    .iter()
                    .enumerate()
                    .map_if_ok(|(num, t)| {
                        self.convert_type_from_ffi(t, format!("arg{}", num), false, false)
                    })?
                    .join(", ");
                writeln!(
                    self,
                    include_str!("../templates/crate/closure_slot_wrapper.rs.in"),
                    type_name = rust_struct.path.full_name(Some(&self.crate_name)),
                    pub_type_name = rust_struct.path.last(),
                    callback_name = self.rust_path_to_string(&slot_wrapper.callback_path),
                    args = args,
                    args_tuple = args_tuple,
                    connections_mod = connections_mod,
                    object_type_name = object_type_name,
                    func_args = func_args,
                    callback_args = callback_args,
                    cpp_args = cpp_args
                )?;
            }
            RustStructKind::FfiClassType(_) => {
                writeln!(
                    self,
                    "{}struct {} {{ _unused: u8, }}",
                    visibility,
                    rust_struct.path.last()
                )?;
            }
            RustStructKind::SizedType(_) => {
                bail!("sized struct can't be generated with rust code generator")
            }
        }

        if database.children(&rust_struct.path).next().is_some() {
            writeln!(self, "impl {} {{", rust_struct.path.last())?;
            self.generate_children(&rust_struct.path, database)?;
            writeln!(self, "}}")?;
            writeln!(self)?;
        }

        Ok(())
    }

    fn generate_enum_value(&mut self, value: &RustEnumValue) -> Result<()> {
        writeln!(
            self,
            "{}",
            format_doc(&doc_formatter::enum_value_doc(value))
        )?;
        let struct_path =
            self.rust_path_to_string(&value.path.parent().expect("enum value must have parent"));
        writeln!(
            self,
            "pub const {value_name}: {struct_path} = {struct_path}({value});",
            value_name = value.path.last(),
            struct_path = struct_path,
            value = value.value
        )?;
        Ok(())
    }

    // TODO: generate relative paths for better readability
    fn rust_path_to_string(&self, path: &RustPath) -> String {
        path.full_name(Some(&self.crate_name))
    }

    /// Generates Rust code containing declaration of a FFI function `func`.
    fn rust_ffi_function_to_code(&self, func: &RustFFIFunction) -> String {
        let mut args = func.arguments.iter().map(|arg| {
            format!(
                "{}: {}",
                arg.name,
                self.rust_type_to_code(&arg.argument_type)
            )
        });
        format!(
            "  pub fn {}({}){};\n",
            func.path.last(),
            args.join(", "),
            match func.return_type {
                RustType::EmptyTuple => String::new(),
                _ => format!(" -> {}", self.rust_type_to_code(&func.return_type)),
            }
        )
    }

    /// Wraps `expression` of type `type1.rust_ffi_type` to convert
    /// it to type `type1.rust_api_type`.
    /// If `in_unsafe_context` is `true`, the output code will be placed inside
    /// an `unsafe` block.
    /// If `use_ffi_result_var` is `true`, the output code will assign
    /// the value to a temporary variable `ffi_result` and return it.
    fn convert_type_from_ffi(
        &self,
        type1: &CompleteType,
        expression: String,
        in_unsafe_context: bool,
        use_ffi_result_var: bool,
    ) -> Result<String> {
        let (unsafe_start, unsafe_end) = if in_unsafe_context {
            ("", "")
        } else {
            ("unsafe { ", " }")
        };
        if type1.rust_api_to_ffi_conversion == RustToFfiTypeConversion::None {
            return Ok(expression);
        }

        let (code1, source_expr) = if use_ffi_result_var {
            (
                format!("let ffi_result = {};\n", expression),
                "ffi_result".to_string(),
            )
        } else {
            (String::new(), expression)
        };
        let code2 = match type1.rust_api_to_ffi_conversion {
            RustToFfiTypeConversion::None => unreachable!(),
            RustToFfiTypeConversion::RefToPtr | RustToFfiTypeConversion::OptionRefToPtr => {
                let api_is_const = if type1.rust_api_to_ffi_conversion
                    == RustToFfiTypeConversion::OptionRefToPtr
                {
                    if let RustType::Common {
                        ref generic_arguments,
                        ..
                    } = type1.rust_api_type
                    {
                        let args = generic_arguments
                            .as_ref()
                            .ok_or_else(|| err_msg("Option with no generic_arguments"))?;
                        if args.len() != 1 {
                            bail!("Option with invalid args count");
                        }
                        args[0].last_is_const()?
                    } else {
                        bail!("Option type expected");
                    }
                } else {
                    type1.rust_api_type.last_is_const()?
                };
                let unwrap_code = match type1.rust_api_to_ffi_conversion {
                    RustToFfiTypeConversion::RefToPtr => {
                        ".expect(\"Attempted to convert null pointer to reference\")"
                    }
                    RustToFfiTypeConversion::OptionRefToPtr => "",
                    _ => unreachable!(),
                };
                format!(
                    "{unsafe_start}{}.{}(){unsafe_end}{}",
                    source_expr,
                    if api_is_const { "as_ref" } else { "as_mut" },
                    unwrap_code,
                    unsafe_start = unsafe_start,
                    unsafe_end = unsafe_end
                )
            }
            RustToFfiTypeConversion::ValueToPtr => format!(
                "{unsafe_start}*{}{unsafe_end}",
                source_expr,
                unsafe_start = unsafe_start,
                unsafe_end = unsafe_end
            ),
            RustToFfiTypeConversion::CppBoxToPtr => format!(
                "{unsafe_start}::cpp_utils::CppBox::new({}){unsafe_end}",
                source_expr,
                unsafe_start = unsafe_start,
                unsafe_end = unsafe_end
            ),
            RustToFfiTypeConversion::QFlagsToUInt => {
                let mut qflags_type = type1.rust_api_type.clone();
                if let RustType::Common {
                    ref mut generic_arguments,
                    ..
                } = qflags_type
                {
                    *generic_arguments = None;
                } else {
                    unreachable!();
                }
                format!(
                    "{}::from_int({} as i32)",
                    self.rust_type_to_code(&qflags_type),
                    source_expr
                )
            }
        };
        Ok(code1 + &code2)
    }

    /// Generates Rust code for calling an FFI function from a wrapper function.
    /// If `in_unsafe_context` is `true`, the output code will be placed inside
    /// an `unsafe` block.
    fn generate_ffi_call(
        &self,
        arguments: &[RustFunctionArgument],
        return_type: &CompleteType,
        wrapper_data: &RustFfiWrapperData,
        in_unsafe_context: bool,
    ) -> Result<String> {
        let (unsafe_start, unsafe_end) = if in_unsafe_context {
            ("", "")
        } else {
            ("unsafe { ", " }")
        };
        let mut final_args = Vec::new();
        final_args.resize(wrapper_data.cpp_ffi_function.arguments.len(), None);
        let all_args: Vec<RustFunctionArgument> = Vec::from(arguments);
        for arg in &all_args {
            assert!(arg.ffi_index < final_args.len());
            let mut code = arg.name.clone();
            match arg.argument_type.rust_api_to_ffi_conversion {
                RustToFfiTypeConversion::None => {}
                RustToFfiTypeConversion::OptionRefToPtr => {
                    bail!("OptionRefToPtr is not supported here yet");
                }
                RustToFfiTypeConversion::RefToPtr => {
                    if arg.argument_type.rust_api_type.is_const()?
                        && !arg.argument_type.rust_ffi_type.is_const()?
                    {
                        let mut intermediate_type = arg.argument_type.rust_ffi_type.clone();
                        intermediate_type.set_const(true)?;
                        code = format!(
                            "{} as {} as {}",
                            code,
                            self.rust_type_to_code(&intermediate_type),
                            self.rust_type_to_code(&arg.argument_type.rust_ffi_type)
                        );
                    } else {
                        code = format!(
                            "{} as {}",
                            code,
                            self.rust_type_to_code(&arg.argument_type.rust_ffi_type)
                        );
                    }
                }
                RustToFfiTypeConversion::ValueToPtr | RustToFfiTypeConversion::CppBoxToPtr => {
                    let is_const = if let RustType::PointerLike { ref is_const, .. } =
                        arg.argument_type.rust_ffi_type
                    {
                        *is_const
                    } else {
                        unexpected!("void is not expected here at all!");
                    };
                    if arg.argument_type.rust_api_to_ffi_conversion
                        == RustToFfiTypeConversion::CppBoxToPtr
                    {
                        let method = if is_const { "as_ptr" } else { "as_mut_ptr" };
                        code = format!("{}.{}()", code, method);
                    } else {
                        code = format!(
                            "{}{} as {}",
                            if is_const { "&" } else { "&mut " },
                            code,
                            self.rust_type_to_code(&arg.argument_type.rust_ffi_type)
                        );
                    }
                }
                RustToFfiTypeConversion::QFlagsToUInt => {
                    code = format!("{}.to_int() as ::libc::c_uint", code);
                }
            }
            final_args[arg.ffi_index] = Some(code);
        }

        let mut result = Vec::new();
        let mut maybe_result_var_name = None;
        if let Some(ref i) = wrapper_data.return_type_ffi_index {
            let mut return_var_name = "object".to_string();
            let mut ii = 1;
            while arguments.iter().any(|x| x.name == return_var_name) {
                ii += 1;
                return_var_name = format!("object{}", ii);
            }
            let struct_name =
                if return_type.rust_api_to_ffi_conversion == RustToFfiTypeConversion::CppBoxToPtr {
                    if let RustType::Common {
                        ref generic_arguments,
                        ..
                    } = return_type.rust_api_type
                    {
                        let generic_arguments = generic_arguments
                            .as_ref()
                            .ok_or_else(|| err_msg("CppBox must have generic_arguments"))?;
                        let arg = generic_arguments.get(0).ok_or_else(|| {
                            err_msg("CppBox must have non-empty generic_arguments")
                        })?;
                        self.rust_type_to_code(arg)
                    } else {
                        unexpected!("CppBox type expected");
                    }
                } else {
                    self.rust_type_to_code(&return_type.rust_api_type)
                };
            result.push(format!(
                "{{\nlet mut {var}: {t} = {unsafe_start}\
                 ::cpp_utils::new_uninitialized::NewUninitialized::new_uninitialized()\
                 {unsafe_end};\n",
                var = return_var_name,
                t = struct_name,
                unsafe_start = unsafe_start,
                unsafe_end = unsafe_end
            ));
            final_args[*i as usize] = Some(format!("&mut {}", return_var_name));
            maybe_result_var_name = Some(return_var_name);
        }
        let final_args = final_args
            .into_iter()
            .map_if_ok(|x| x.ok_or_else(|| err_msg("ffi argument is missing")))?;

        result.push(format!(
            "{unsafe_start}{}({}){maybe_semicolon}{unsafe_end}",
            self.rust_path_to_string(&wrapper_data.ffi_function_path),
            final_args.join(", "),
            maybe_semicolon = if maybe_result_var_name.is_some() {
                ";"
            } else {
                ""
            },
            unsafe_start = unsafe_start,
            unsafe_end = unsafe_end
        ));
        if let Some(ref name) = maybe_result_var_name {
            result.push(format!("{}\n}}", name));
        }
        let code = result.join("");
        if maybe_result_var_name.is_none() {
            self.convert_type_from_ffi(&return_type, code, in_unsafe_context, true)
        } else {
            Ok(code)
        }
    }

    /// Generates Rust code for declaring a function's arguments.
    fn arg_texts(&self, args: &[RustFunctionArgument], lifetime: Option<&String>) -> Vec<String> {
        args.iter()
            .map(|arg| {
                if &arg.name == "self" {
                    let self_type = match lifetime {
                        Some(lifetime) => arg
                            .argument_type
                            .rust_api_type
                            .with_lifetime(lifetime.clone()),
                        None => arg.argument_type.rust_api_type.clone(),
                    };
                    match self_type {
                        RustType::Common { .. } => "self".to_string(),
                        RustType::PointerLike {
                            ref kind,
                            ref is_const,
                            ..
                        } => {
                            if let RustPointerLikeTypeKind::Reference { ref lifetime } = *kind {
                                let maybe_mut = if *is_const { "" } else { "mut " };
                                match *lifetime {
                                    Some(ref lifetime) => {
                                        format!("&'{} {}self", lifetime, maybe_mut)
                                    }
                                    None => format!("&{}self", maybe_mut),
                                }
                            } else {
                                panic!("invalid self argument type (indirection)");
                            }
                        }
                        _ => {
                            panic!("invalid self argument type (not Common)");
                        }
                    }
                } else {
                    let mut maybe_mut_declaration = "";
                    if let RustType::Common { .. } = arg.argument_type.rust_api_type {
                        if arg.argument_type.rust_api_to_ffi_conversion
                            == RustToFfiTypeConversion::ValueToPtr
                        {
                            if let RustType::PointerLike { ref is_const, .. } =
                                arg.argument_type.rust_ffi_type
                            {
                                if !*is_const {
                                    maybe_mut_declaration = "mut ";
                                }
                            }
                        }
                    }

                    format!(
                        "{}{}: {}",
                        maybe_mut_declaration,
                        arg.name,
                        match lifetime {
                            Some(lifetime) => self.rust_type_to_code(
                                &arg.argument_type
                                    .rust_api_type
                                    .with_lifetime(lifetime.clone(),)
                            ),
                            None => self.rust_type_to_code(&arg.argument_type.rust_api_type),
                        }
                    )
                }
            })
            .collect()
    }

    /// Generates complete code of a Rust wrapper function.
    fn generate_rust_final_function(&mut self, func: &RustFunction) -> Result<()> {
        let maybe_pub = match func.scope {
            RustFunctionScope::TraitImpl => "",
            _ => "pub ",
        };
        let maybe_unsafe = if func.is_unsafe { "unsafe " } else { "" };

        let body = match func.kind {
            RustFunctionKind::FfiWrapper(ref data) => {
                self.generate_ffi_call(&func.arguments, &func.return_type, data, func.is_unsafe)?
            }
            RustFunctionKind::CppDeletableImpl { ref deleter } => self.rust_path_to_string(deleter),
            RustFunctionKind::SignalOrSlotGetter { .. } => unimplemented!(),
        };

        let return_type_for_signature = if func.return_type.rust_api_type == RustType::EmptyTuple {
            String::new()
        } else {
            format!(
                " -> {}",
                self.rust_type_to_code(&func.return_type.rust_api_type)
            )
        };
        let all_lifetimes: Vec<_> = func
            .arguments
            .iter()
            .filter_map(|x| x.argument_type.rust_api_type.lifetime())
            .collect();
        let lifetimes_text = if all_lifetimes.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                all_lifetimes.iter().map(|x| format!("'{}", x)).join(", ")
            )
        };

        writeln!(
            self,
            "{doc}{maybe_pub}{maybe_unsafe}fn {name}{lifetimes_text}({args}){return_type} \
             {{\n{body}}}\n\n",
            doc = format_doc(&doc_formatter::function_doc(&func)),
            maybe_pub = maybe_pub,
            maybe_unsafe = maybe_unsafe,
            lifetimes_text = lifetimes_text,
            name = func.path.last(),
            args = self.arg_texts(&func.arguments, None).join(", "),
            return_type = return_type_for_signature,
            body = body
        )?;
        Ok(())
    }

    fn generate_lib_file(&mut self, database: &RustDatabase) -> Result<()> {
        if let Some(ref doc) = database.lib_extra_doc {
            writeln!(self, "{}", format_doc_extended(doc, true))?;
        }

        // some ffi functions are not used because
        // some Rust methods are filtered
        // TODO: ideally dead_code shouldn't be needed
        writeln!(
            self,
            "\
             #[allow(dead_code)]\nmod ffi {{ \ninclude!(concat!(env!(\"OUT_DIR\"), \
             \"/ffi.rs\")); \n}}\n",
        )?;
        writeln!(
            self,
            "\
             mod _types {{ \ninclude!(concat!(env!(\"OUT_DIR\"), \
             \"/_types.rs\")); \n}}\n",
        )?;

        let root = RustPath::from_parts(vec![self.crate_name.clone()]);
        self.generate_children(&root, database)?;
        // TODO: generate const with current lib version
        Ok(())
    }

    fn generate_children(&mut self, parent: &RustPath, database: &RustDatabase) -> Result<()> {
        for item in database.children(&parent) {
            self.generate_item(item, database)?;
        }
        // TODO: somehow add items from crate template
        Ok(())
    }

    fn generate_trait_impl(&mut self, trait1: &RustTraitImpl) -> Result<()> {
        let associated_types_text = trait1
            .associated_types
            .iter()
            .map(|t| format!("type {} = {};", t.name, self.rust_type_to_code(&t.value)))
            .join("\n");

        writeln!(
            self,
            "impl {} for {} {{\n{}",
            self.rust_type_to_code(&trait1.trait_type),
            self.rust_type_to_code(&trait1.target_type),
            associated_types_text,
        )?;

        for func in &trait1.functions {
            self.generate_rust_final_function(func)?;
        }

        writeln!(self, "}}\n")?;
        Ok(())
    }

    /// Generates `ffi.in.rs` file.
    fn generate_ffi_code(&mut self, functions: &[(String, Vec<RustFFIFunction>)]) -> Result<()> {
        let mut code = String::new();
        code.push_str("extern \"C\" {\n");
        for &(ref include_file, ref functions) in functions {
            code.push_str(&format!("  // Header: {}\n", include_file));
            for function in functions {
                code.push_str(&self.rust_ffi_function_to_code(function));
            }
            code.push_str("\n");
        }
        code.push_str("}\n");
        writeln!(self, "{}", code)?;
        Ok(())
    }
}

pub fn generate_lib_file(
    crate_name: &str,
    database: &RustDatabase,
    destination: impl Write,
) -> Result<()> {
    let mut generator = Generator {
        crate_name: crate_name.to_string(),
        destination,
    };
    generator.generate_lib_file(database)?;
    Ok(())
}

pub fn generate_ffi_file(
    crate_name: &str,
    _database: &RustDatabase,
    destination: impl Write,
) -> Result<()> {
    let mut generator = Generator {
        crate_name: crate_name.to_string(),
        destination,
    };
    // TODO: Rust FFI functions storage?
    generator.generate_ffi_code(&[])?;
    Ok(())
}

// TODO: reimplement impl FlaggableEnum
/*
    if *is_flaggable {
        r = r + &format!(
            include_str!("../templates/crate/impl_flaggable.rs.in"),
            name = type1.name.last_name()?,
            trait_type = RustName::new(vec![
                "qt_core".to_string(),
                "flags".to_string(),
                "FlaggableEnum".to_string(),
            ])?
            .full_name(Some(&self.config.crate_properties.name()))
        );
    }
*/

// TODO: reimplement impl Receiver for raw slot wrapper
/*
if let Some(ref slot_wrapper) = *slot_wrapper {
    let arg_texts: Vec<_> = slot_wrapper
        .arguments
        .iter()
        .map(|t| self.rust_type_to_code(&t.rust_api_type))
        .collect();
    let args = arg_texts.join(", ");
    let args_tuple = format!(
        "{}{}",
        args,
        if arg_texts.len() == 1 { "," } else { "" }
    );
    let connections_mod = RustName::new(vec![
        "qt_core".to_string(),
        "connection".to_string(),
    ])?
    .full_name(Some(&self.config.crate_properties.name()));
    let object_type_name = RustName::new(vec![
        "qt_core".to_string(),
        "object".to_string(),
        "Object".to_string(),
    ])?
    .full_name(Some(&self.config.crate_properties.name()));
    r.push_str(&format!(
        include_str!(
            "../templates/crate/extern_slot_impl_receiver.rs.in"
        ),
        type_name = type1
            .name
            .full_name(Some(&self.config.crate_properties.name())),
        args_tuple = args_tuple,
        receiver_id = slot_wrapper.receiver_id,
        connections_mod = connections_mod,
        object_type_name = object_type_name
    ));
}
*/
