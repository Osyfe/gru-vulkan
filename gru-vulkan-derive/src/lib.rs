extern crate proc_macro;

use crate::proc_macro::TokenStream;

fn attribute_group_derive(input: TokenStream, rate: proc_macro2::TokenStream) -> TokenStream
{
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let mut comps = quote::quote!();
    if let syn::Data::Struct(data) = input.data
    {
        if let syn::Fields::Named(fields) = data.fields
        {
            for field in fields.named
            {
                if !field.ident.is_some() { panic!("Only named fields allowed."); }
                if field.attrs.len() != 1 { panic!("Only the pattern \"location = ?\" allowed."); }
                let attr = &field.attrs[0];
                if attr.path().is_ident("location")
                {
                    if let syn::Meta::NameValue(syn::MetaNameValue { value: syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(value), .. }), ..}) = &attr.meta
                    {
                        let location: u32 = value.base10_parse().expect("Only the pattern \"location = ?\" allowed.");
                        if let syn::Type::Path(path) = &field.ty
                        {
                            let ty = &path.path.segments.iter().nth(0).expect("Only the pattern \"location = ?\" allowed.").ident;
                            comps.extend(quote::quote!((AttributeLocation(#location), #ty::TYPE),));
                        } else { panic!("Only the pattern \"location = ?\" allowed."); }
                    }
                }
            }
        } else { panic!("Only named fields allowed."); }
    } else { panic!("Only structs allowed."); }
    let expanded = quote::quote!
    {
        impl AttributeGroupReprCpacked for #name
        {
            const RATE: InputRate = InputRate::#rate;
            const ATTRIBUTES: &'static [(AttributeLocation, AttributeType)] = &[#comps];
        }
    };
    //println!("{}", expanded);
    TokenStream::from(expanded)
}

#[proc_macro_derive(VertexAttributeGroupReprCpacked, attributes(location))]
pub fn vertex_attribute_group_derive(input: TokenStream) -> TokenStream
{
    attribute_group_derive(input, quote::quote!(Vertex))
}

#[proc_macro_derive(InstanceAttributeGroupReprCpacked, attributes(location))]
pub fn instance_attribute_group_derive(input: TokenStream) -> TokenStream
{
    attribute_group_derive(input, quote::quote!(Instance))
}

#[proc_macro_derive(DescriptorStructReprC)]
pub fn descriptor_struct_derive(input: TokenStream) -> TokenStream
{
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let expanded = quote::quote!
    {
        impl DescriptorStructReprC for #name { }
    };
    //println!("{}", expanded);
    TokenStream::from(expanded)
}

#[proc_macro_derive(AttributeType)]
pub fn attribute_type_derive(input: TokenStream) -> TokenStream
{
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let expanded = quote::quote!
    {
        impl #name
        {
            pub const TYPE: AttributeType = AttributeType::#name;
        }
    };
    //println!("{}", expanded);
    TokenStream::from(expanded)
}
