import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { useEngineStore } from "@/stores/useEngineStore";

export function ThroughputChart() {
  const chartData = useEngineStore((s) => s.chartData);

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-sm text-muted-foreground">
        Waiting for data...
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={280}>
      <AreaChart
        data={chartData}
        margin={{ top: 5, right: 20, left: 10, bottom: 5 }}
      >
        <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
        <XAxis
          dataKey="elapsed_s"
          tick={{ fontSize: 11 }}
          label={{
            value: "Time (s)",
            position: "insideBottom",
            offset: -5,
            fontSize: 11,
          }}
        />
        <YAxis
          tick={{ fontSize: 11 }}
          label={{
            value: "req/s",
            angle: -90,
            position: "insideLeft",
            fontSize: 11,
          }}
        />
        <Tooltip contentStyle={{ fontSize: 12 }} />
        <Area
          type="monotone"
          dataKey="rps"
          name="Throughput"
          stroke="#22c55e"
          fill="#22c55e"
          fillOpacity={0.15}
          strokeWidth={2}
          dot={false}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}
