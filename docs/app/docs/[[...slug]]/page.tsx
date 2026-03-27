import { docsSource as source } from "@/app/source";
import {
  DocsPage,
  DocsBody,
  DocsDescription,
  DocsTitle,
} from "fumadocs-ui/page";
import { notFound } from "next/navigation";
import defaultMdxComponents from "fumadocs-ui/mdx";
import { ImageZoom } from "fumadocs-ui/components/image-zoom";
import { Accordion, Accordions } from "fumadocs-ui/components/accordion";
import { Step, Steps } from "fumadocs-ui/components/steps";
import { Tab, Tabs } from "fumadocs-ui/components/tabs";
import { Callout } from "fumadocs-ui/components/callout";
import { TypeTable } from "fumadocs-ui/components/type-table";
import { Files, Folder, File } from "fumadocs-ui/components/files";
import { getPageTreePeers } from "fumadocs-core/page-tree";
import { GithubIcon } from "@/app/components/icons";
import {
  MarkdownCopyButton,
  ViewOptionsPopover,
} from "@/components/ai/page-actions";

export default async function Page(props: {
  params: Promise<{ slug?: string[] }>;
}) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;
  const markdownUrl = `${page.url}.mdx`;
  const githubUrl = getGithubUrl(page.path);

  return (
    <DocsPage
      toc={page.data.toc.filter((item) => item.depth <= 3)}
      full={page.data.full}
      tableOfContent={{ footer: <EditOnGithub path={page.path} /> }}
    >
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <div className="flex flex-row gap-2 items-center border-b pt-2 pb-6">
        <MarkdownCopyButton markdownUrl={markdownUrl} />
        <ViewOptionsPopover
          markdownUrl={markdownUrl}
          githubUrl={githubUrl}
        />
      </div>
      <DocsBody>
        <MDX
          components={{
            ...defaultMdxComponents,
            img: (props) => <ImageZoom {...(props as React.ImgHTMLAttributes<HTMLImageElement> & { src: string })} />,
            Accordion,
            Accordions,
            Step,
            Steps,
            Tab,
            Tabs,
            Callout,
            TypeTable,
            Files,
            Folder,
            File,
          }}
        />
        {page.data.index ? (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            {getPageTreePeers(source.pageTree, page.url).map((item) => (
              <a
                key={item.url}
                href={item.url}
                className="block rounded-lg border p-4 transition-colors hover:bg-fd-accent"
              >
                <p className="font-medium">{item.name}</p>
                {item.description ? (
                  <p className="mt-1 text-sm text-fd-muted-foreground">
                    {item.description}
                  </p>
                ) : null}
              </a>
            ))}
          </div>
        ) : null}
      </DocsBody>
    </DocsPage>
  );
}

function getGithubUrl(path: string) {
  return `https://github.com/solana-foundation/anchor/blob/master/docs/content/docs/${path.startsWith("/") ? path.slice(1) : path}`;
}

function EditOnGithub({ path }: { path: string }) {
  return (
    <a
      href={getGithubUrl(path)}
      target="_blank"
      rel="noreferrer noopener"
      className="pt-2 flex items-center gap-2 text-sm text-fd-muted-foreground hover:text-fd-accent-foreground/80"
    >
      <GithubIcon width="18" height="18" />
      <span>Edit on GitHub</span>
    </a>
  );
}

export async function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>;
}) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  return {
    title: page.data.title,
    description: page.data.description,
  };
}
