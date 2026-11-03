static class SceneInputs {
    public static string Root() {
        var configured=Environment.GetEnvironmentVariable("TOWER_SCENE_SOURCE");
        if(!string.IsNullOrWhiteSpace(configured)) {
            var root=Path.GetFullPath(configured);
            if(!File.Exists(Path.Combine(root,"boss-pool.txt")))throw new Exception("TOWER_SCENE_SOURCE must contain boss-pool.txt");
            return root;
        }
        foreach(var start in new[]{Directory.GetCurrentDirectory(),AppContext.BaseDirectory})
            for(var directory=new DirectoryInfo(start);directory!=null;directory=directory.Parent) {
                var root=Path.Combine(directory.FullName,"scene-assets");
                if(File.Exists(Path.Combine(directory.FullName,"Cargo.toml"))&&File.Exists(Path.Combine(root,"boss-pool.txt")))return root;
            }
        throw new Exception("Cannot locate repository scene-assets; set TOWER_SCENE_SOURCE explicitly");
    }
}
