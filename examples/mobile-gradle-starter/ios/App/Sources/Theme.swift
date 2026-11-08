import SwiftUI

public enum JirauTheme {
    public static let primaryColor = Color(red: 0.12, green: 0.34, blue: 0.19)
    public static let accentColor = Color(red: 0.16, green: 0.32, blue: 0.75)
    public static let earthColor = Color(red: 0.82, green: 0.49, blue: 0.18)
    public static let cornerRadius: CGFloat = 14.0
}

public struct JirauCardModifier: ViewModifier {
    public func body(content: Content) -> some View {
        content
            .padding()
            .background(Color(.secondarySystemBackground))
            .clipShape(RoundedRectangle(cornerRadius: JirauTheme.cornerRadius, style: .continuous))
            .shadow(color: Color.black.opacity(0.04), radius: 6, x: 0, y: 3)
    }
}

public extension View {
    func jirauCardStyle() -> some View {
        modifier(JirauCardModifier())
    }
}
