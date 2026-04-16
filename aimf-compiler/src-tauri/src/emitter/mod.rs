use std::fmt::Write;

use crate::types::{AimfDocument, NavEntry, CtxEntry};

/// Emit a complete AIMF v2 document as a string.
pub fn emit(doc: &AimfDocument) -> String {
    let mut out = String::with_capacity(4096);

    emit_nav(&mut out, &doc.nav);
    emit_ctx(&mut out, &doc.ctx);

    out
}

/// Emit the @NAV section.
fn emit_nav(out: &mut String, nav: &[NavEntry]) {
    let _ = writeln!(out, "@NAV");

    if nav.is_empty() {
        let _ = writeln!(out);
        return;
    }

    // Calculate column widths for alignment
    let id_w = nav.iter().map(|n| n.id.len()).max().unwrap_or(4).max(4);
    let type_w = 4; // "SPEC" is longest at 4
    let path_w = nav.iter().map(|n| n.path.to_string_lossy().len()).max().unwrap_or(4).max(4);
    let about_w = nav.iter().map(|n| n.about.len()).max().unwrap_or(5).max(5);
    let tok_w = nav.iter().map(|n| format!("{}", n.tokens).len()).max().unwrap_or(6).max(6);

    // Header line
    let _ = writeln!(
        out,
        "{:<id_w$} | {:<type_w$} | {:<path_w$} | {:<about_w$} | {:>tok_w$} | load",
        "id", "type", "path", "about", "tokens",
        id_w = id_w, type_w = type_w, path_w = path_w, about_w = about_w, tok_w = tok_w,
    );

    // Data lines
    for entry in nav {
        let path_str = entry.path.to_string_lossy();
        // Append / for DIR entries if not already there
        let display_path = if entry.resource_type == crate::types::ResourceType::DIR
            && !path_str.ends_with('/')
        {
            format!("{}/", path_str)
        } else {
            path_str.to_string()
        };

        let hints = entry.load_hints.join(", ");

        let _ = writeln!(
            out,
            "{:<id_w$} | {:<type_w$} | {:<path_w$} | {:<about_w$} | {:>tok_w$} | {}",
            entry.id,
            entry.resource_type.as_str(),
            display_path,
            entry.about,
            entry.tokens,
            hints,
            id_w = id_w, type_w = type_w, path_w = path_w, about_w = about_w, tok_w = tok_w,
        );
    }

    let _ = writeln!(out);
}

/// Emit the @CTX section.
fn emit_ctx(out: &mut String, ctx: &[CtxEntry]) {
    let _ = writeln!(out, "@CTX");

    for entry in ctx {
        let _ = writeln!(out, "{}: {}", entry.key, entry.value);
    }
}

/// Emit a task handoff document: AIMF header + markdown instructions.
pub fn emit_task(
    nav: &[NavEntry],
    ctx: &[CtxEntry],
    task_title: &str,
    task_body: &str,
) -> String {
    let mut out = String::with_capacity(8192);

    // AIMF sections
    let doc = AimfDocument {
        nav: nav.to_vec(),
        ctx: ctx.to_vec(),
    };
    out.push_str(&emit(&doc));

    // Token budget summary
    let total: usize = nav.iter().map(|n| n.tokens).sum();
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "<!-- Total resource cost: ~{} tokens -->", total);
    let _ = writeln!(out);

    // Task markdown
    let _ = writeln!(out, "# Task: {}", task_title);
    let _ = writeln!(out);
    out.push_str(task_body);

    out
}
