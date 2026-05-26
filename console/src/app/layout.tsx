import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { Providers } from "@/components/providers";
import { Topbar } from "@/components/topbar";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "Sandbox Console",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className={`${inter.className} bg-bg text-text antialiased min-h-screen`}>
        <Providers>
          <Topbar />
          <main>{children}</main>
        </Providers>
      </body>
    </html>
  );
}
