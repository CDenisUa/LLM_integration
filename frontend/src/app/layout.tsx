// Core
import type { Metadata } from "next";
// Components
import { ThemeProvider } from "@/components/ThemeProvider";
import Sidebar from "@/components/layout/Sidebar";
// Styles
import "./globals.css";

export const metadata: Metadata = {
  title: "LLM Lab",
  description: "Learning playground for LLM integrations",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="antialiased" suppressHydrationWarning>
      <body className="bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 min-h-screen">
        <ThemeProvider>
          <div className="flex min-h-screen">
            <Sidebar />
            <main className="flex-1 overflow-auto">
              {children}
            </main>
          </div>
        </ThemeProvider>
      </body>
    </html>
  );
}
