import { docsSource } from "@/app/source";
import { getLLMText } from "@/lib/get-llm-text";

export const revalidate = false;

export async function GET() {
  const pages = docsSource.getPages().map(getLLMText);
  const content = await Promise.all(pages);

  return new Response(content.join("\n\n"), {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
  });
}
