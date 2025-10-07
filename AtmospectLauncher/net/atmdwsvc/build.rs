fn main() {
    let mut res = winres::WindowsResource::new();
    res.set("CompanyName", "Forgotten Private Coalition");
    res.set("FileDescription", "Atmospect Homing Web Service");
    res.set("ProductName", "Atmospect Launcher");
    res.set("FileVersion", "1.1.0.0");
    res.set("ProductVersion", "1.0.0.0");
    res.set("OriginalFilename", "atmdwsvc.exe");
    res.set("InternalName", "atmdwsvc.exe");
    res.set("LegalCopyright", "© 2025 Forgotten Private Coalition. All rights reserved.");
    res.set("LegalTrademarks", "Forgotten TM");
    res.set("Comments", "Atmospect Web Work Service");

    res.set("Language", "0x0409");
    res.set("CodePage", "1200");

    res.compile().unwrap();
}
