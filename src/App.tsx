import { AppLayout } from "@/components/layout/AppLayout";
import { RequestPage } from "@/pages/RequestPage";
import { TestPlansPage } from "@/pages/TestPlansPage";
import { ResultsPage } from "@/pages/ResultsPage";
import { WebSocketPage } from "@/pages/WebSocketPage";
import { GraphQLPage } from "@/pages/GraphQLPage";
import { useAppStore } from "@/stores/useAppStore";
import { useEngineEvents } from "@/hooks/useEngineEvents";

function ActivePage() {
  const { activeView } = useAppStore();

  // Subscribe to all Tauri engine events globally so they are always captured
  useEngineEvents();

  switch (activeView) {
    case "request":
      return <RequestPage />;
    case "websocket":
      return <WebSocketPage />;
    case "graphql":
      return <GraphQLPage />;
    case "test-plans":
      return <TestPlansPage />;
    case "results":
      return <ResultsPage />;
  }
}

export default function App() {
  return (
    <AppLayout>
      <ActivePage />
    </AppLayout>
  );
}
