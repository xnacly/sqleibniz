use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn trace(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_sig = &input.sig;
    let fn_block = &input.block;

    let expanded = quote! {
        #fn_vis #fn_sig {
            #[cfg(feature = "trace")]
            CALL_DEPTH.with(|depth| {
                let indent = depth.get();
                println!("{}↳ {} | {:?}", " ".repeat(indent), stringify!(#fn_name), self.cur().ttype);
                depth.set(indent + 1);
            });

            let result = { #fn_block };

            #[cfg(feature = "trace")]
            CALL_DEPTH.with(|depth| {
                let indent = depth.get();
                depth.set(indent.saturating_sub(1));
            });

            result
        }
    };

    TokenStream::from(expanded)
}
