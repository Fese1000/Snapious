import Foundation

/// Language of the interface: English (default) or German. The source strings in the code are German;
/// `tr` returns the English text from `Translations.en` when English is selected.
enum L10n {
    static var language: String = UserDefaults.standard.string(forKey: "language") ?? "en" {
        didSet { UserDefaults.standard.set(language, forKey: "language") }
    }

    /// Format specifiers (`%@`, `%d`, `%.1f`, `%lld` …) found in a string, without `%%`.
    private static func specs(_ s: String) -> [String] {
        var out: [String] = []
        let chars = Array(s)
        var i = 0
        while i < chars.count {
            if chars[i] == "%" {
                if i + 1 < chars.count, chars[i + 1] == "%" { i += 2; continue }
                var j = i + 1
                while j < chars.count, !"@dDiuUfFeEgGsScC".contains(chars[j]) , "0123456789.-+ #l$".contains(chars[j]) { j += 1 }
                if j < chars.count, "@dDiuUfFeEgGsScC".contains(chars[j]) {
                    out.append(String(chars[i...j])); i = j + 1; continue
                }
            }
            i += 1
        }
        return out
    }

    /// The key with every specifier replaced by `%@` (how `Translations.en` is keyed).
    private static func normalized(_ s: String) -> String {
        var out = ""
        let chars = Array(s)
        var i = 0
        while i < chars.count {
            if chars[i] == "%" {
                if i + 1 < chars.count, chars[i + 1] == "%" { out += "%%"; i += 2; continue }
                var j = i + 1
                while j < chars.count, "0123456789.-+ #l$".contains(chars[j]) { j += 1 }
                if j < chars.count, "@dDiuUfFeEgGsScC".contains(chars[j]) { out += "%@"; i = j + 1; continue }
            }
            out.append(chars[i]); i += 1
        }
        return out
    }

    static func translate(_ key: String) -> String {
        guard language == "en" else { return key }
        guard let t = Translations.en[normalized(key)] else { return key }
        // put the original specifiers (%d, %.1f …) back, in order
        let sp = specs(key)
        if sp.isEmpty { return t }
        var out = ""
        var idx = 0
        let chars = Array(t)
        var i = 0
        while i < chars.count {
            if chars[i] == "%", i + 1 < chars.count, chars[i + 1] == "@" {
                out += idx < sp.count ? sp[idx] : "%@"; idx += 1; i += 2; continue
            }
            if chars[i] == "%", i + 1 < chars.count, chars[i + 1] == "%" { out += "%%"; i += 2; continue }
            out.append(chars[i]); i += 1
        }
        return out
    }
}

/// Translates a user-facing string and fills in `%@` placeholders with `args`.
func tr(_ key: String, _ args: String...) -> String {
    let t = L10n.translate(key)
    guard !args.isEmpty else { return t }
    return String(format: t, arguments: args.map { $0 as CVarArg })
}
