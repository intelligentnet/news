use std::collections::HashMap;

pub fn make_call(call: &str, paras: HashMap<&str, &str>) -> String {
    let mut call = call.to_string();
    let mut sep = if call.contains('?') { "&" } else { "?" };

    for (k, v) in paras {
        if !v.is_empty() {
            let pv = format!("{sep}{k}={v}");
            call = format!("{call}{pv}");
            sep = "&";
        }
    }

    call
}
