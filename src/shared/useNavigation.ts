import { useState } from "react";
import type { Lang } from "./i18n";

export type PageId = "overview" | "library" | "agents" | "workspaces" | "deploy" | "operations";

export type NavState = {
  page: PageId;
  lang: Lang;
  setLang: (lang: Lang) => void;
  navigate: (page: PageId) => void;
};

export function useNavigation(): NavState {
  const [page, setPage] = useState<PageId>("overview");
  const [lang, setLang] = useState<Lang>("zh");

  return {
    page,
    lang,
    setLang,
    navigate: setPage,
  };
}
