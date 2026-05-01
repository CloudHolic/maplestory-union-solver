import { Board } from "@/components/board/Board.tsx";
import { BoardControls } from "@/components/board/BoardControls.tsx";
import { CharacterPanel } from "@/components/characters/CharacterPanel.tsx";

function App() {
	return (
		<div className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
			<div className="flex flex-col items-center gap-3 lg:flex-row lg:items-start lg:justify-center">
				<div className="flex w-[55vmin] max-w-175 flex-col gap-2">
					<Board />
					<BoardControls />
				</div>
				<div className="w-full lg:w-105 lg:shrink-5">
					<CharacterPanel />
				</div>
			</div>
		</div>
	);
}

export default App;
