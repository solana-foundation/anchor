// inkeep apiKeys is public, set to only work on https://www.anchor-lang.com
export const settings = {
  baseSettings: {
    apiKey: "815f8fee7a5da7d98c681626dfbd268bdf7f7dc7cb80f618",
    integrationId: "cm7nvgfuu001ks6016t3m0gje",
    organizationId: "org_iKCh36NfndEFpB3M",
    organizationDisplayName: "Anchor",
    primaryBrandColor: "#9945ff",
    customCardSettings: [
      {
        filters: {
          UrlMatch: {
            ruleType: "PartialUrl",
            partialUrl: "https://www.anchor-lang.com/docs",
          },
        },
        searchTabLabel: "Anchor Docs",
      },
      {
        filters: {
          UrlMatch: {
            ruleType: "PartialUrl",
            partialUrl: "solana.com",
          },
        },
        searchTabLabel: "Solana Docs",
      },
      {
        filters: {
          UrlMatch: {
            ruleType: "PartialUrl",
            partialUrl: "https://docs.anza.xyz/",
          },
        },
        searchTabLabel: "Anza Docs",
      },
      {
        filters: {
          UrlMatch: {
            ruleType: "PartialUrl",
            partialUrl: "https://solana.stackexchange.com/",
          },
        },
        searchTabLabel: "Stack Exchange",
      },
    ],
  },
  searchSettings: {
    shouldOpenLinksInNewTab: true,
    placeholder: "Search",
  },
  aiChatSettings: {
    chatSubjectName: "Solana",
    introMessage:
      "I'm an AI assistant trained on documentation, github repos, and other content. Ask me anything about `Solana`.",
    botAvatarSrcUrl: "https://solana.com/favicon.png",
    disclaimerSettings: {
      isDisclaimerEnabled: true,
    },
    actionButtonLabels: {
      getHelpButtonLabel: "Get Support",
    },
    getHelpCallToActions: [
      {
        name: "Discord",
        url: "https://discord.com/invite/NHHGSXAnXk",
        icon: {
          builtIn: "FaDiscord",
        },
      },
      {
        name: "Stack Exchange",
        url: "https://solana.stackexchange.com/",
        icon: {
          builtIn: "FaStackOverflow",
        },
      },
    ],
    quickQuestions: [
      "How to set up local environment for Solana development?",
      "What is the Solana Account Model?",
      "How to build a Solana Program?",
    ],
  },
};
