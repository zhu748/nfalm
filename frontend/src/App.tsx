import "./App.css";
import { getVersion } from "./api";
import { useState, useEffect } from "react";

function App() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    // Fetch and set the version when component mounts
    getVersion().then((v) => setVersion(v));
  }, []);

  return (
    <>
      <h1>ClewdR</h1>
      <h2>{version}</h2>
      <div className="card">
        <p>
          Edit <code>src/App.tsx</code> and save to test HMR
        </p>
      </div>
    </>
  );
}

export default App;
